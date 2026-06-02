use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use crate::state::AppState;
use crate::ws::{self, TableUpdate};

use crate::components::{ConfirmPopup, ConfirmStyle, DataTable, DATA_TABLE_PAGE_SIZE, ErrorMessage, Icon, IconName, PageLayout, TableActionButton, TableActionVariant};
use crate::Route;

#[derive(Debug, Clone)]
struct ScheduledTask {
    table_name: String,
    scheduled_id: String,
    scheduled_at_raw: serde_json::Value,
    /// Sortable key: micros for Time, 0 for Interval (show intervals first)
    sort_key: u64,
}

#[component]
pub fn ScheduledTasks(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut tasks = use_signal(Vec::<ScheduledTask>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut page = use_signal(|| 0usize);
    let mut stream_status = use_signal(|| "Connecting...".to_string());
    // Task removal ("Stop") state.
    let mut stopping_task = use_signal(|| Option::<(String, String)>::None);
    let mut stop_saving = use_signal(|| false);
    let mut stop_error = use_signal(|| Option::<String>::None);
    let mut trigger_stop = use_signal(|| 0u32);
    let mut now_secs = use_signal(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });

    // Tick every second so countdowns update
    use_future(move || async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            now_secs.set(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    });

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    // Subscribe to schedule tables via WebSocket
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let db_identity = db_identity.clone();
            spawn(async move {
                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    loading.set(false);
                    stream_status.set("Disconnected".to_string());
                    return;
                };
                let base_url = state.server_url.clone();
                let token = if state.token.is_empty() { None } else { Some(state.token.clone()) };
                drop(state);

                // Discover schedule tables by checking which have a scheduled_at column
                let schema = match api.get_schema(&db_identity).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to fetch schema: {e}");
                        error_msg.set(Some(format!("Failed to load schema: {e}")));
                        loading.set(false);
                        stream_status.set("Error".to_string());
                        return;
                    }
                };

                let mut schedule_tables = Vec::new();
                for table in &schema.tables {
                    let query = format!("SELECT * FROM {} LIMIT 1", table.name);
                    if let Ok(result) = api.execute_sql(&db_identity, &query).await
                        && result.columns.iter().any(|c| c == "scheduled_at")
                    {
                        schedule_tables.push(table.name.clone());
                    }
                }

                if schedule_tables.is_empty() {
                    log::info!("No schedule tables found");
                    loading.set(false);
                    stream_status.set("Live".to_string());
                    return;
                }

                log::info!("Subscribing to schedule tables: {:?}", schedule_tables);

                // Fetch column info for each schedule table
                let mut table_columns: Vec<(String, Vec<String>)> = Vec::new();
                for table_name in &schedule_tables {
                    let query = format!("SELECT * FROM {table_name} LIMIT 1");
                    if let Ok(result) = api.execute_sql(&db_identity, &query).await {
                        table_columns.push((table_name.clone(), result.columns));
                    }
                }

                // Subscribe to each table individually and merge into a shared channel
                let (merge_tx, mut merge_rx) = tokio::sync::mpsc::unbounded_channel::<TableUpdate>();

                for table_name in &schedule_tables {
                    let mut rx = ws::subscribe_to_table(
                        &base_url,
                        &db_identity,
                        table_name,
                        token.as_deref(),
                    );
                    let merge_tx = merge_tx.clone();
                    tokio::spawn(async move {
                        while let Some(update) = rx.recv().await {
                            if merge_tx.send(update).is_err() {
                                break;
                            }
                        }
                    });
                }
                drop(merge_tx); // Drop original sender so channel closes when all forwarders finish

                stream_status.set("Connected".to_string());
                let mut received_initial = 0usize;
                let total_tables = schedule_tables.len();

                // Coalesce the (potentially very high frequency) stream of updates.
                // We keep an authoritative buffer keyed by (table, scheduled_id) and
                // only push to the reactive `tasks` signal on a fixed cadence, so the
                // renderer is never flooded regardless of how many updates arrive.
                let mut buffer: HashMap<(String, String), ScheduledTask> = HashMap::new();
                let mut dirty = false;
                let mut initial_flushed = false;
                let mut applied_since_flush = 0usize;
                let mut flush = tokio::time::interval(tokio::time::Duration::from_millis(250));
                flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        biased;
                        maybe_update = merge_rx.recv() => {
                            let Some(update) = maybe_update else { break; };
                            match update {
                                TableUpdate::InitialRows { table_name: tbl, rows: initial_rows, .. } => {
                                    let columns = table_columns.iter()
                                        .find(|(n, _)| *n == tbl)
                                        .map(|(_, c)| c.as_slice())
                                        .unwrap_or(&[]);
                                    let new_tasks = parse_rows_for_table(&tbl, &initial_rows, columns);
                                    log::info!("Received {} initial rows for {tbl}", new_tasks.len());
                                    for t in new_tasks {
                                        buffer.insert((t.table_name.clone(), t.scheduled_id.clone()), t);
                                    }
                                    dirty = true;
                                    received_initial += 1;
                                }
                                TableUpdate::Insert { table_name: tbl, rows: new_rows, .. } => {
                                    let columns = table_columns.iter()
                                        .find(|(n, _)| *n == tbl)
                                        .map(|(_, c)| c.as_slice())
                                        .unwrap_or(&[]);
                                    let new_tasks = parse_rows_for_table(&tbl, &new_rows, columns);
                                    for t in new_tasks {
                                        buffer.insert((t.table_name.clone(), t.scheduled_id.clone()), t);
                                    }
                                    dirty = true;
                                    applied_since_flush += 1;
                                }
                                TableUpdate::Delete { table_name: tbl, rows: del_rows, .. } => {
                                    let columns = table_columns.iter()
                                        .find(|(n, _)| *n == tbl)
                                        .map(|(_, c)| c.as_slice())
                                        .unwrap_or(&[]);
                                    let del_tasks = parse_rows_for_table(&tbl, &del_rows, columns);
                                    for d in del_tasks {
                                        buffer.remove(&(d.table_name, d.scheduled_id));
                                    }
                                    dirty = true;
                                    applied_since_flush += 1;
                                }
                                TableUpdate::Error(e) => {
                                    log::error!("Scheduled tasks subscription error: {e}");
                                    error_msg.set(Some(e));
                                    stream_status.set("Disconnected".to_string());
                                    loading.set(false);
                                    break;
                                }
                            }

                            // Flush immediately once all initial loads are in, so the
                            // first paint isn't delayed by the debounce interval.
                            if !initial_flushed && received_initial >= total_tables {
                                let mut snapshot: Vec<ScheduledTask> = buffer.values().cloned().collect();
                                snapshot.sort_by_key(|t| t.sort_key);
                                tasks.set(snapshot);
                                dirty = false;
                                applied_since_flush = 0;
                                initial_flushed = true;
                                loading.set(false);
                                stream_status.set("Live".to_string());
                            }
                        }
                        _ = flush.tick() => {
                            if dirty && initial_flushed {
                                if applied_since_flush > 0 {
                                    log::debug!("Applying {applied_since_flush} coalesced schedule updates");
                                }
                                let mut snapshot: Vec<ScheduledTask> = buffer.values().cloned().collect();
                                snapshot.sort_by_key(|t| t.sort_key);
                                tasks.set(snapshot);
                                dirty = false;
                                applied_since_flush = 0;
                            }
                        }
                    }
                }

                // If we exit the loop without an error, it means all WS connections closed
                if stream_status.read().as_str() != "Disconnected" && stream_status.read().as_str() != "Error" {
                    log::warn!("Scheduled tasks: all WebSocket connections ended");
                    stream_status.set("Disconnected".to_string());
                }
            });
        }
    });

    // Remove a scheduled task once the user confirms the "Stop" action.
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            if *trigger_stop.read() == 0 {
                return;
            }
            let Some((table_name, scheduled_id)) = stopping_task.read().clone() else {
                return;
            };
            let db_identity = db_identity.clone();
            spawn(async move {
                stop_saving.set(true);
                stop_error.set(None);

                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    stop_saving.set(false);
                    return;
                };
                drop(state);

                let id_value = format_id_value(&scheduled_id);
                let sql = format!("DELETE FROM {table_name} WHERE scheduled_id = {id_value}");
                log::info!("Stopping scheduled task: {sql}");
                match api.execute_sql(&db_identity, &sql).await {
                    Ok(_) => {
                        // The live subscription delivers the matching Delete update,
                        // which removes the row from the table.
                        stopping_task.set(None);
                    }
                    Err(e) => {
                        log::error!("Failed to stop scheduled task: {e}");
                        stop_error.set(Some(e));
                    }
                }
                stop_saving.set(false);
            });
        }
    });

    let readonly = app_state.read().readonly;
    let all_tasks = tasks.read();
    let total_rows = all_tasks.len();
    let current_page = *page.read();
    let start = current_page * DATA_TABLE_PAGE_SIZE;
    let end = (start + DATA_TABLE_PAGE_SIZE).min(total_rows);

    let header_status_color = match stream_status.read().as_str() {
        "Live" => "text-emerald-600 bg-emerald-500/10",
        "Error" => "text-red-600 bg-red-500/10",
        "Disconnected" => "text-gray-500 bg-gray-500/10",
        _ => "text-yellow-600 bg-yellow-500/10",
    };

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Scheduled",
            title: "Scheduled Tasks".to_string(),
            header_extra: Some(rsx! {
                span { class: "inline-flex items-center gap-1.5 text-xs font-medium px-2.5 py-1 rounded-full {header_status_color}",
                    Icon { name: IconName::Circle, class: "w-1.5 h-1.5" }
                    "{stream_status}"
                }
            }),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col gap-4",
                if *loading.read() {
                    div { class: "text-gray-500 text-sm", "Loading..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    ErrorMessage { message: err.clone() }
                }

                if !*loading.read() && all_tasks.is_empty() {
                    div { class: "flex-1 flex items-center justify-center",
                        div { class: "text-center",
                            div { class: "text-gray-500 text-sm mb-2", "No scheduled tasks found" }
                            div { class: "text-gray-600 text-xs",
                                "Schedule tables contain a "
                                code { class: "text-gray-400", "scheduled_at" }
                                " column."
                            }
                        }
                    }
                }

                if !*loading.read() && !all_tasks.is_empty() {
                    DataTable {
                        columns: vec![
                            ("Table".to_string(), String::new()),
                            ("ID".to_string(), String::new()),
                            ("Scheduled At".to_string(), String::new()),
                            (String::new(), "text-right".to_string()),
                        ],
                        total_rows,
                        current_page,
                        on_page_change: move |p: usize| page.set(p),
                        for task in all_tasks[start..end].iter() {
                            tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors",
                                td { class: "px-4 py-2.5 text-sm text-gray-400 whitespace-nowrap",
                                    "{task.table_name}"
                                }
                                td { class: "px-4 py-2.5 text-sm text-gray-300 whitespace-nowrap font-mono",
                                    "{task.scheduled_id}"
                                }
                                td { class: "px-4 py-2.5 text-sm text-gray-300 whitespace-nowrap",
                                    {format_scheduled_at(&task.scheduled_at_raw)}
                                    {
                                        let now = *now_secs.read();
                                        match behind_secs(&task.scheduled_at_raw, now) {
                                            Some(behind) if behind > 60 => rsx! {
                                                span { class: "inline-flex items-center gap-1 text-xs font-medium text-red-300 bg-red-500/10 px-1.5 py-0.5 rounded ml-2",
                                                    Icon { name: IconName::Clock, class: "w-3 h-3" }
                                                    {format_behind(behind)}
                                                }
                                            },
                                            _ => rsx! {
                                                span { class: "text-xs text-yellow-600 ml-1", {format_countdown(&task.scheduled_at_raw, now)} }
                                            },
                                        }
                                    }
                                }
                                td { class: "px-4 py-2.5 whitespace-nowrap text-right",
                                    {
                                        let tbl = task.table_name.clone();
                                        let id = task.scheduled_id.clone();
                                        rsx! {
                                            TableActionButton {
                                                label: "Stop".to_string(),
                                                icon: IconName::Trash,
                                                variant: TableActionVariant::Danger,
                                                disabled: readonly,
                                                title: if readonly { "Enable write mode to stop tasks".to_string() } else { "Stop this scheduled task".to_string() },
                                                onclick: move |_| {
                                                    stop_error.set(None);
                                                    stopping_task.set(Some((tbl.clone(), id.clone())));
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some((tbl, id)) = stopping_task.read().clone() {
                    ConfirmPopup {
                        title: "Stop Scheduled Task".to_string(),
                        message: format!("This will permanently remove scheduled task {id} from {tbl}. Continue?"),
                        confirm_label: "Stop Task".to_string(),
                        style: ConfirmStyle::Danger,
                        loading: *stop_saving.read(),
                        error: stop_error.read().clone(),
                        on_cancel: move |_| {
                            stopping_task.set(None);
                            stop_error.set(None);
                        },
                        on_confirm: move |_| {
                            trigger_stop.set(trigger_stop() + 1);
                        },
                    }
                }
            }
        }
    }
}

/// Parse raw JSON rows from a known table into ScheduledTask structs.
/// Rows may be JSON objects (keyed by column name) or arrays (positional).
fn parse_rows_for_table(
    table_name: &str,
    rows: &[serde_json::Value],
    columns: &[String],
) -> Vec<ScheduledTask> {
    let id_col_idx = columns.iter().position(|c| c == "scheduled_id");
    let at_col_idx = columns.iter().position(|c| c == "scheduled_at");

    let mut tasks = Vec::new();
    for row in rows {
        let scheduled_id;
        let scheduled_at_raw;

        if let Some(obj) = row.as_object() {
            // Object format: {"scheduled_id": ..., "scheduled_at": ...}
            scheduled_id = obj
                .get("scheduled_id")
                .map(format_value)
                .unwrap_or_default();
            scheduled_at_raw = obj
                .get("scheduled_at")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        } else if let Some(arr) = row.as_array() {
            // Array format: positional based on column order
            scheduled_id = id_col_idx
                .and_then(|i| arr.get(i))
                .map(format_value)
                .unwrap_or_default();
            scheduled_at_raw = at_col_idx
                .and_then(|i| arr.get(i))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
        } else {
            continue;
        }

        let sort_key = extract_sort_key(&scheduled_at_raw);

        tasks.push(ScheduledTask {
            table_name: table_name.to_string(),
            scheduled_id,
            scheduled_at_raw,
            sort_key,
        });
    }

    tasks
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| format!("{other:?}")),
    }
}

/// Format a `scheduled_id` for use in a SQL WHERE clause. Numeric ids are emitted
/// bare (preserving full precision for large integer keys); anything else is
/// quoted and single-quote escaped.
fn format_id_value(id: &str) -> String {
    if id.parse::<i128>().is_ok() || id.parse::<u128>().is_ok() {
        id.to_string()
    } else {
        let escaped = id.replace('\'', "''");
        format!("'{escaped}'")
    }
}

/// Extract a sortable key from the scheduled_at value.
/// Intervals get sort_key 0, Times get the microsecond timestamp.
fn extract_sort_key(value: &serde_json::Value) -> u64 {
    if let Some((variant, micros)) = parse_schedule_at(value) {
        return match variant {
            0 => 0,            // Interval: sort first
            1 => micros,       // Time: sort by timestamp
            _ => 0,
        };
    }
    0
}

/// Extract microseconds from a value that might be:
/// - a number directly
/// - an array [number]
/// - an object {"0": number} (product type)
/// - an array of arrays [[number]]
fn extract_micros(value: &serde_json::Value) -> u64 {
    // Direct number
    if let Some(n) = value.as_u64() {
        return n;
    }
    if let Some(n) = value.as_i64() {
        return n as u64;
    }

    // Array: [number] or [[number]]
    if let Some(arr) = value.as_array()
        && let Some(first) = arr.first() {
            if let Some(n) = first.as_u64() {
                return n;
            }
            if let Some(n) = first.as_i64() {
                return n as u64;
            }
            // Nested array: [[number]]
            if let Some(inner_arr) = first.as_array()
                && let Some(inner) = inner_arr.first() {
                    if let Some(n) = inner.as_u64() {
                        return n;
                    }
                    if let Some(n) = inner.as_i64() {
                        return n as u64;
                    }
                }
        }

    // Object product type: {"0": number} or {"microseconds": number}
    if let Some(obj) = value.as_object() {
        // Try "0" key (positional product)
        if let Some(v) = obj.get("0") {
            if let Some(n) = v.as_u64() {
                return n;
            }
            if let Some(n) = v.as_i64() {
                return n as u64;
            }
        }
        // Try "microseconds" key
        if let Some(v) = obj.get("microseconds") {
            if let Some(n) = v.as_u64() {
                return n;
            }
            if let Some(n) = v.as_i64() {
                return n as u64;
            }
        }
        // Try first numeric value in the object
        for v in obj.values() {
            if let Some(n) = v.as_u64() {
                return n;
            }
            if let Some(n) = v.as_i64() {
                return n as u64;
            }
        }
    }

    0
}

fn format_scheduled_at(value: &serde_json::Value) -> String {
    // Parse the ScheduleAt sum type from various possible encodings
    if let Some((variant, micros)) = parse_schedule_at(value) {
        return match variant {
            0 => format_interval_micros(micros),
            1 => format_time_micros(micros),
            _ => format!("{value}"),
        };
    }

    match value {
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| format!("{other:?}")),
    }
}

/// Parse a ScheduleAt value from various SATS-JSON encodings.
/// Returns (variant_tag, microseconds) or None if unrecognized.
/// Supports:
///   - Array: [variant, [micros]]
///   - Tagged object: {"tag": variant, "value": ...}  (WS format)
///   - Named object: {"Time": micros} or {"Interval": micros}
fn parse_schedule_at(value: &serde_json::Value) -> Option<(u64, u64)> {
    // Array format from SQL: [variant_index, [value]]
    if let Some(arr) = value.as_array()
        && arr.len() == 2
            && let Some(variant) = arr[0].as_u64() {
                let micros = extract_micros(&arr[1]);
                return Some((variant, micros));
            }

    // Object formats from WebSocket
    if let Some(obj) = value.as_object() {
        // Tagged format: {"tag": N, "value": ...}
        if let Some(tag) = obj.get("tag").and_then(|t| t.as_u64()) {
            let inner = obj.get("value").unwrap_or(&serde_json::Value::Null);
            let micros = extract_micros(inner);
            return Some((tag, micros));
        }

        // Named format: {"Time": v} or {"Interval": v}
        if let Some(v) = obj.get("Time").or_else(|| obj.get("time")) {
            return Some((1, extract_micros(v)));
        }
        if let Some(v) = obj.get("Interval").or_else(|| obj.get("interval")) {
            return Some((0, extract_micros(v)));
        }
    }

    None
}

fn format_time_micros(micros: u64) -> String {
    let secs = micros / 1_000_000;
    
    chrono_format_epoch_secs(secs)
}

fn format_interval_micros(micros: u64) -> String {
    if micros >= 3_600_000_000 {
        let hours = micros as f64 / 3_600_000_000.0;
        format!("Every {hours:.1}h")
    } else if micros >= 60_000_000 {
        let mins = micros as f64 / 60_000_000.0;
        format!("Every {mins:.1}m")
    } else if micros >= 1_000_000 {
        let secs = micros as f64 / 1_000_000.0;
        format!("Every {secs:.1}s")
    } else if micros >= 1_000 {
        let ms = micros as f64 / 1_000.0;
        format!("Every {ms:.0}ms")
    } else {
        format!("Every {micros}us")
    }
}

/// Format a countdown string for the task. Returns empty for intervals or past times.
fn format_countdown(value: &serde_json::Value, now_secs: u64) -> String {
    if let Some((variant, micros)) = parse_schedule_at(value) {
        if variant == 1 {
            // Time variant: show countdown
            let target_secs = micros / 1_000_000;
            if target_secs > now_secs {
                let remaining = target_secs - now_secs;
                let days = remaining / 86400;
                let hours = (remaining % 86400) / 3600;
                let mins = (remaining % 3600) / 60;
                let secs = remaining % 60;

                if days > 0 {
                    format!("in {days}d {hours}h {mins}m")
                } else if hours > 0 {
                    format!("in {hours}h {mins}m {secs}s")
                } else if mins > 0 {
                    format!("in {mins}m {secs}s")
                } else {
                    format!("in {secs}s")
                }
            } else {
                "overdue".to_string()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

/// Returns how many seconds a Time-variant scheduled task is behind its target
/// time, or `None` if it is an interval, scheduled in the future, or unparseable.
fn behind_secs(value: &serde_json::Value, now_secs: u64) -> Option<u64> {
    let (variant, micros) = parse_schedule_at(value)?;
    if variant != 1 {
        return None;
    }
    let target_secs = micros / 1_000_000;
    (now_secs > target_secs).then(|| now_secs - target_secs)
}

/// Format a "behind by" duration in compact form for the warning badge.
fn format_behind(behind: u64) -> String {
    let days = behind / 86400;
    let hours = (behind % 86400) / 3600;
    let mins = (behind % 3600) / 60;
    let secs = behind % 60;
    if days > 0 {
        format!("{days}d {hours}h behind")
    } else if hours > 0 {
        format!("{hours}h {mins}m behind")
    } else if mins > 0 {
        format!("{mins}m {secs}s behind")
    } else {
        format!("{secs}s behind")
    }
}

/// Format epoch seconds as a human-readable local datetime string.
fn chrono_format_epoch_secs(secs: u64) -> String {
    use chrono::{Local, TimeZone};

    match Local.timestamp_opt(secs as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => format!("{secs}s (invalid timestamp)"),
    }
}
