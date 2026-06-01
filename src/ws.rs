use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite};

/// Messages sent from client to server over WebSocket (SATS-JSON externally tagged format).
#[derive(Serialize)]
enum ClientMessage {
    SubscribeSingle(SubscribeSinglePayload),
}

#[derive(Serialize)]
struct SubscribeSinglePayload {
    query: String,
    request_id: u32,
    /// QuerySetId is a SATS newtype tuple, serialized as [u32] in JSON.
    query_id: (u32,),
}

/// Represents a row update (insert or delete) for the UI.
#[derive(Debug, Clone)]
pub enum TableUpdate {
    /// Initial set of all rows matching the subscription.
    InitialRows {
        #[allow(dead_code)]
        table_name: String,
        rows: Vec<serde_json::Value>,
    },
    /// Rows inserted by a transaction.
    Insert {
        #[allow(dead_code)]
        table_name: String,
        rows: Vec<serde_json::Value>,
    },
    /// Rows deleted by a transaction.
    Delete {
        #[allow(dead_code)]
        table_name: String,
        rows: Vec<serde_json::Value>,
    },
    /// An error occurred.
    Error(String),
}

/// Subscribe to a table via WebSocket and receive live updates.
/// Returns a channel receiver that yields TableUpdate messages.
pub fn subscribe_to_table(
    base_url: &str,
    db_identity: &str,
    table_name: &str,
    token: Option<&str>,
) -> mpsc::UnboundedReceiver<TableUpdate> {
    let (tx, rx) = mpsc::unbounded_channel();

    let ws_url = build_ws_url(base_url, db_identity);
    let query = format!("SELECT * FROM {}", table_name);
    let token = token.map(|t| t.to_string());
    let table_name = table_name.to_string();

    tokio::spawn(async move {
        if let Err(e) = run_subscription(ws_url, vec![query], token, table_name, tx.clone()).await {
            log::error!("WebSocket subscription error: {e}");
            let _ = tx.send(TableUpdate::Error(e));
        }
    });

    rx
}

fn build_ws_url(base_url: &str, db_identity: &str) -> String {
    let ws_base = if base_url.starts_with("https://") {
        base_url.replacen("https://", "wss://", 1)
    } else if base_url.starts_with("http://") {
        base_url.replacen("http://", "ws://", 1)
    } else {
        format!("ws://{}", base_url)
    };
    format!("{}/v1/database/{}/subscribe", ws_base.trim_end_matches('/'), db_identity)
}

async fn run_subscription(
    ws_url: String,
    queries: Vec<String>,
    token: Option<String>,
    table_name: String,
    tx: mpsc::UnboundedSender<TableUpdate>,
) -> Result<(), String> {
    log::info!("Connecting WebSocket to: {ws_url}");

    let uri: http::Uri = ws_url.parse().map_err(|e| format!("Invalid URL: {e}"))?;
    let host = uri
        .authority()
        .map(|a| a.as_str().to_string())
        .unwrap_or_default();

    let mut request = http::Request::builder()
        .uri(&ws_url)
        .header("Host", &host)
        .header("Sec-WebSocket-Protocol", "v1.json.spacetimedb")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        );

    if let Some(ref token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let request = request
        .body(())
        .map_err(|e| format!("Failed to build request: {e}"))?;

    let (ws_stream, _response) = connect_async(request)
        .await
        .map_err(|e| format!("WebSocket connection failed: {e}"))?;

    log::info!("WebSocket connected, subscribing to: {:?}", queries);

    let (mut write, mut read) = ws_stream.split();

    // Send SubscribeSingle messages (one per query, each with a unique query_id)
    for (i, query) in queries.iter().enumerate() {
        let subscribe_msg = ClientMessage::SubscribeSingle(SubscribeSinglePayload {
            query: query.clone(),
            request_id: (i as u32) + 1,
            query_id: ((i as u32) + 1,),
        });
        let msg_json = serde_json::to_string(&subscribe_msg)
            .map_err(|e| format!("Failed to serialize subscribe message: {e}"))?;
        write
            .send(tungstenite::Message::Text(msg_json.into()))
            .await
            .map_err(|e| format!("Failed to send subscribe: {e}"))?;
    }

    // Read messages – keep `write` alive so the underlying socket can flush pong frames
    // (tokio-tungstenite auto-responds to pings inside poll_next via write_pending).
    let _write_handle = write;

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(tungstenite::Error::Protocol(e)) => {
                log::warn!("[ws] protocol error (non-fatal): {e}");
                continue;
            }
            Err(e) => {
                log::error!("[ws] read error: {e}");
                let _ = tx.send(TableUpdate::Error(format!("Connection lost: {e}")));
                break;
            }
        };

        match msg {
            tungstenite::Message::Text(text) => {
                log::debug!("[ws] received text ({} bytes)", text.len());
                if let Err(e) = handle_server_message(&text, &table_name, &tx) {
                    log::warn!("[ws] failed to parse server message: {e}");
                }
            }
            tungstenite::Message::Binary(data) => {
                log::debug!("[ws] received binary ({} bytes), ignoring", data.len());
            }
            tungstenite::Message::Ping(_) => {
                log::debug!("[ws] received ping (auto-pong handled by tungstenite)");
            }
            tungstenite::Message::Pong(_) => {
                log::debug!("[ws] received pong");
            }
            tungstenite::Message::Close(frame) => {
                log::info!("[ws] closed by server: {:?}", frame);
                break;
            }
            tungstenite::Message::Frame(_) => {}
        }
    }

    log::info!("[ws] subscription loop ended for table: {table_name}");
    Ok(())
}

fn handle_server_message(
    text: &str,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) -> Result<(), String> {
    let msg: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("JSON parse error: {e}"))?;

    // The server message is an externally-tagged enum.
    if let Some(initial) = msg.get("InitialSubscription") {
        log::info!("[ws] received InitialSubscription for {table_name}");
        handle_initial_subscription(initial, table_name, tx);
    } else if let Some(applied) = msg.get("SubscribeApplied") {
        log::info!("[ws] received SubscribeApplied for {table_name}");
        handle_subscribe_applied(applied, table_name, tx);
    } else if let Some(update) = msg.get("TransactionUpdate") {
        log::debug!("[ws] received TransactionUpdate for {table_name}");
        log::debug!("[ws] TransactionUpdate content: {}", &text[..text.len().min(500)]);
        handle_transaction_update(update, table_name, tx);
    } else if let Some(update) = msg.get("TransactionUpdateLight") {
        log::debug!("[ws] received TransactionUpdateLight for {table_name}");
        log::debug!("[ws] TransactionUpdateLight content: {}", &text[..text.len().min(500)]);
        handle_transaction_update_light(update, table_name, tx);
    } else if let Some(error) = msg.get("SubscriptionError") {
        let err_msg = error
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown subscription error");
        log::error!("[ws] SubscriptionError for {table_name}: {err_msg}");
        let _ = tx.send(TableUpdate::Error(err_msg.to_string()));
    } else if msg.get("IdentityToken").is_some() {
        log::debug!("[ws] received IdentityToken");
    } else {
        log::warn!("[ws] unhandled message type for {table_name}: {}", &text[..text.len().min(200)]);
    }

    Ok(())
}

fn handle_initial_subscription(
    msg: &serde_json::Value,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) {
    if let Some(db_update) = msg.get("database_update") {
        let rows = extract_inserts_from_database_update(db_update, table_name);
        let _ = tx.send(TableUpdate::InitialRows {
            table_name: table_name.to_string(),
            rows,
        });
    }
}

fn handle_subscribe_applied(
    msg: &serde_json::Value,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) {
    if let Some(rows_obj) = msg.get("rows")
        && let Some(table_rows) = rows_obj.get("table_rows")
    {
        let rows = extract_inserts_from_table_update(table_rows);
        let _ = tx.send(TableUpdate::InitialRows {
            table_name: table_name.to_string(),
            rows,
        });
    }
}

fn handle_transaction_update(
    msg: &serde_json::Value,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) {
    if let Some(status) = msg.get("status")
        && let Some(committed) = status.get("Committed")
    {
        log::debug!("[ws] TransactionUpdate status=Committed for {table_name}");
        process_database_update(committed, table_name, tx);
    } else {
        log::warn!("[ws] TransactionUpdate has unexpected status structure for {table_name}: {}", 
            serde_json::to_string(msg).unwrap_or_default().chars().take(300).collect::<String>());
    }
}

fn handle_transaction_update_light(
    msg: &serde_json::Value,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) {
    if let Some(update) = msg.get("update") {
        process_database_update(update, table_name, tx);
    }
}

fn process_database_update(
    db_update: &serde_json::Value,
    table_name: &str,
    tx: &mpsc::UnboundedSender<TableUpdate>,
) {
    let inserts = extract_inserts_from_database_update(db_update, table_name);
    let deletes = extract_deletes_from_database_update(db_update, table_name);

    log::debug!("[ws] database update for {table_name}: {} inserts, {} deletes", inserts.len(), deletes.len());

    if !deletes.is_empty() {
        let _ = tx.send(TableUpdate::Delete {
            table_name: table_name.to_string(),
            rows: deletes,
        });
    }
    if !inserts.is_empty() {
        let _ = tx.send(TableUpdate::Insert {
            table_name: table_name.to_string(),
            rows: inserts,
        });
    }
}

fn extract_inserts_from_database_update(
    db_update: &serde_json::Value,
    _table_name: &str,
) -> Vec<serde_json::Value> {
    let mut all_rows = Vec::new();
    if let Some(tables) = db_update.get("tables").and_then(|t| t.as_array()) {
        for table in tables {
            if let Some(updates) = table.get("updates").and_then(|u| u.as_array()) {
                for update in updates {
                    // Handle both compressed and uncompressed formats
                    let query_update = if let Some(uncompressed) = update.get("Uncompressed") {
                        uncompressed
                    } else {
                        update
                    };
                    if let Some(inserts) = query_update.get("inserts").and_then(|i| i.as_array()) {
                        for row in inserts {
                            // In JSON format, each row is a JSON string that needs parsing
                            if let Some(row_str) = row.as_str() {
                                if let Ok(parsed) = serde_json::from_str(row_str) {
                                    all_rows.push(parsed);
                                }
                            } else {
                                all_rows.push(row.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    all_rows
}

fn extract_deletes_from_database_update(
    db_update: &serde_json::Value,
    _table_name: &str,
) -> Vec<serde_json::Value> {
    let mut all_rows = Vec::new();
    if let Some(tables) = db_update.get("tables").and_then(|t| t.as_array()) {
        for table in tables {
            if let Some(updates) = table.get("updates").and_then(|u| u.as_array()) {
                for update in updates {
                    let query_update = if let Some(uncompressed) = update.get("Uncompressed") {
                        uncompressed
                    } else {
                        update
                    };
                    if let Some(deletes) = query_update.get("deletes").and_then(|d| d.as_array()) {
                        for row in deletes {
                            if let Some(row_str) = row.as_str() {
                                if let Ok(parsed) = serde_json::from_str(row_str) {
                                    all_rows.push(parsed);
                                }
                            } else {
                                all_rows.push(row.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    all_rows
}

fn extract_inserts_from_table_update(table_update: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut all_rows = Vec::new();
    if let Some(updates) = table_update.get("updates").and_then(|u| u.as_array()) {
        for update in updates {
            let query_update = if let Some(uncompressed) = update.get("Uncompressed") {
                uncompressed
            } else {
                update
            };
            if let Some(inserts) = query_update.get("inserts").and_then(|i| i.as_array()) {
                for row in inserts {
                    if let Some(row_str) = row.as_str() {
                        if let Ok(parsed) = serde_json::from_str(row_str) {
                            all_rows.push(parsed);
                        }
                    } else {
                        all_rows.push(row.clone());
                    }
                }
            }
        }
    }
    all_rows
}
