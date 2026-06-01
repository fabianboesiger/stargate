use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::{self, ApiClient, DatabaseEntry};
use crate::components::{Button, Icon, IconName, TextInput, Select, SelectOption, ErrorMessage};
use crate::Route;

#[component]
pub fn Login() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();

    let default_url = app_state.read().default_server_url().unwrap_or_default();
    let has_token = app_state.read().cli_token().is_some();

    let mut server_url = use_signal(|| default_url.clone());
    let mut token = use_signal(String::new);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut connecting = use_signal(|| false);
    let mut use_cli_creds = use_signal(|| has_token);
    let mut custom_server = use_signal(|| false);
    let mut oauth_status = use_signal(|| Option::<String>::None);
    let mut custom_auth_host = use_signal(|| false);
    let mut auth_host_url = use_signal(|| String::from("https://spacetimedb.com"));

    // Database selection state (shown after successful connect)
    let mut databases = use_signal(Vec::<DatabaseEntry>::new);
    let mut connected_identity = use_signal(|| Option::<String>::None);

    let server_options: Vec<SelectOption> = {
        let state = app_state.read();
        let opts: Vec<SelectOption> = state
            .available_servers()
            .iter()
            .map(|s| SelectOption {
                value: s.url(),
                label: format!("{} ({})", s.nickname, s.url()),
            })
            .collect();
        opts
    };

    let handle_connect = move |_| {
        let url = server_url.read().clone();
        let tok = if *use_cli_creds.read() {
            let state = app_state.read();
            state.cli_token().unwrap_or("").to_string()
        } else {
            token.read().clone()
        };

        if url.is_empty() {
            error_msg.set(Some("Server URL is required".into()));
            return;
        }
        if tok.is_empty() {
            error_msg.set(Some("Token is required".into()));
            return;
        }

        connecting.set(true);
        error_msg.set(None);

        spawn(async move {
            let api = ApiClient::new(&url, Some(&tok));
            let identity = extract_identity_from_token(&tok);

            match api.list_databases(&identity).await {
                Ok(dbs) => {
                    app_state.write().connect(&url, &tok, &identity);
                    connected_identity.set(Some(identity));
                    databases.set(dbs);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Connection failed: {e}")));
                }
            }
            connecting.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen flex items-center justify-center bg-gray-950 p-4",
            div { class: "bg-gray-900 border border-gray-800 rounded-2xl shadow-2xl p-8 w-full max-w-md",
                // Logo
                div { class: "flex justify-center mb-6",
                    Icon {
                        name: IconName::Logo,
                        class: "w-12 h-12 text-blue-400",
                    }
                }
                h1 { class: "text-2xl font-bold text-center text-white mb-1", "Stargate" }
                p { class: "text-gray-500 text-center text-sm mb-8",
                    "Connect to your SpacetimeDB instance"
                }

                // Show database selection if connected, otherwise show login form
                if connected_identity.read().is_some() {
                    // Database selection
                    div { class: "mb-5",
                        div { class: "flex items-center justify-between mb-3",
                            label { class: "text-sm font-medium text-gray-400", "Select a database" }
                            button {
                                class: "text-xs text-gray-500 hover:text-gray-300 transition-colors",
                                onclick: move |_| {
                                    connected_identity.set(None);
                                    databases.set(Vec::new());
                                    *app_state.write() = AppState::new();
                                },
                                "Disconnect"
                            }
                        }

                        if databases.read().is_empty() {
                            p { class: "text-gray-500 text-sm italic",
                                "No databases found for this identity."
                            }
                        } else {
                            div { class: "flex flex-col gap-1 max-h-64 overflow-y-auto",
                                for db in databases.read().iter() {
                                    DatabaseOption { db: db.clone() }
                                }
                            }
                        }
                    }
                } else {
                    // Login form
                    // Server selection
                    div { class: "mb-5",
                        label { class: "block text-sm font-medium text-gray-400 mb-2",
                            "Server"
                        }
                        if !server_options.is_empty() && !*custom_server.read() {
                            Select {
                                options: server_options.clone(),
                                value: server_url.read().clone(),
                                onchange: move |evt: FormEvent| {
                                    let val = evt.value();
                                    server_url.set(val);
                                },
                            }
                        }
                        if server_options.is_empty() || *custom_server.read() {
                            TextInput {
                                value: server_url.read().clone(),
                                oninput: move |evt: FormEvent| server_url.set(evt.value()),
                                placeholder: "http://localhost:3000",
                            }
                        }
                        if !server_options.is_empty() {
                            label { class: "flex items-center gap-2 mt-2 cursor-pointer select-none",
                                input {
                                    r#type: "checkbox",
                                    class: "w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-600 focus:ring-blue-500/20",
                                    checked: *custom_server.read(),
                                    onchange: move |evt: FormEvent| {
                                        let checked = evt.checked();
                                        custom_server.set(checked);
                                        if !checked {
                                            // Reset to first server option when unchecking
                                            if let Some(first) = server_options.first() {
                                                server_url.set(first.value.clone());
                                            }
                                        }
                                    },
                                }
                                span { class: "text-sm text-gray-500", "Custom server" }
                            }
                        }
                    }

                    // Token input
                    div { class: "mb-6",
                        label { class: "block text-sm font-medium text-gray-400 mb-2",
                            "Authentication"
                        }

                        if has_token {
                            div { class: "flex items-center gap-2 mb-3",
                                input {
                                    r#type: "checkbox",
                                    class: "w-4 h-4 rounded border-gray-600 bg-gray-800 text-blue-600 focus:ring-blue-500/20",
                                    checked: *use_cli_creds.read(),
                                    onchange: move |evt: FormEvent| use_cli_creds.set(evt.checked()),
                                }
                                span { class: "text-sm text-gray-500",
                                    "Use CLI credentials (~/.config/spacetime/cli.toml)"
                                }
                            }
                        }

                        if !*use_cli_creds.read() {
                            TextInput {
                                value: token.read().clone(),
                                oninput: move |evt: FormEvent| token.set(evt.value()),
                                placeholder: "Enter your SpacetimeDB token",
                                input_type: "password",
                            }
                            // OAuth login via auth host
                            div { class: "mt-3",
                                button {
                                    class: "w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg border border-gray-700 bg-gray-800 text-sm text-gray-300 hover:bg-gray-700 hover:border-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                                    disabled: oauth_status.read().is_some(),
                                    onclick: move |_| {
                                        let host = auth_host_url.read().clone();
                                        oauth_status.set(Some("Waiting for browser login...".to_string()));
                                        error_msg.set(None);
                                        spawn(async move {
                                            match perform_oauth_login(&host).await {
                                                Ok(obtained_token) => {
                                                    oauth_status.set(None);
                                                    token.set(obtained_token);
                                                }
                                                Err(e) => {
                                                    oauth_status.set(None);
                                                    error_msg.set(Some(e));
                                                }
                                            }
                                        });
                                    },
                                    Icon { name: IconName::RightFromBracket, class: "w-4 h-4" }
                                    if let Some(status) = oauth_status.read().as_ref() {
                                        "{status}"
                                    } else {
                                        "Login via Browser"
                                    }
                                }
                                // Auth host config
                                div { class: "mt-2",
                                    label { class: "flex items-center gap-2 cursor-pointer select-none",
                                        input {
                                            r#type: "checkbox",
                                            class: "w-3.5 h-3.5 rounded border-gray-600 bg-gray-800 text-blue-600 focus:ring-blue-500/20",
                                            checked: *custom_auth_host.read(),
                                            onchange: move |evt: FormEvent| {
                                                let checked = evt.checked();
                                                custom_auth_host.set(checked);
                                                if !checked {
                                                    auth_host_url.set("https://spacetimedb.com".to_string());
                                                }
                                            },
                                        }
                                        span { class: "text-[11px] text-gray-600", "Custom auth host" }
                                    }
                                    if *custom_auth_host.read() {
                                        div { class: "mt-1.5",
                                            TextInput {
                                                value: auth_host_url.read().clone(),
                                                oninput: move |evt: FormEvent| auth_host_url.set(evt.value()),
                                                placeholder: "https://auth.example.com",
                                            }
                                        }
                                    }
                                    if !*custom_auth_host.read() {
                                        p { class: "text-[11px] text-gray-600 mt-1",
                                            "Using spacetimedb.com (GitHub OAuth)"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Error message
                    if let Some(err) = error_msg.read().as_ref() {
                        div { class: "mb-4",
                            ErrorMessage { message: err.clone() }
                        }
                    }

                    // Connect button
                    Button {
                        label: if *connecting.read() { String::from("Connecting...") } else { String::from("Connect") },
                        onclick: handle_connect,
                        disabled: *connecting.read(),
                    }
                }

                p { class: "text-xs text-gray-600 text-center mt-6", "SpacetimeDB Admin UI v0.1.0" }
            }
        }
    }
}

#[component]
fn DatabaseOption(db: DatabaseEntry) -> Element {
    let db_identity = db.identity.clone();
    let display = if !db.names.is_empty() {
        db.names.join(", ")
    } else {
        format!("{}...", &db.identity[..16.min(db.identity.len())])
    };
    let short_id = format!("{}...", &db.identity[..16.min(db.identity.len())]);

    rsx! {
        button {
            class: "w-full text-left px-4 py-3 rounded-lg border border-gray-800 hover:border-blue-500/50 hover:bg-gray-800/50 transition-all group",
            onclick: move |_| {
                navigator()
                    .push(Route::DatabaseTables {
                        db_identity: db_identity.clone(),
                    });
            },
            div { class: "text-sm font-medium text-gray-200 group-hover:text-white",
                "{display}"
            }
            if !db.names.is_empty() {
                div { class: "text-xs text-gray-600 font-mono mt-0.5", "{short_id}" }
            }
        }
    }
}

/// Extract the identity from a SpacetimeDB JWT token.
/// The token payload contains a `hex_identity` field.
fn extract_identity_from_token(token: &str) -> String {
    // If it's already a hex string (32 or 64 chars), return as-is
    if (token.len() == 32 || token.len() == 64) && token.chars().all(|c| c.is_ascii_hexdigit()) {
        return token.to_string();
    }
    // Decode JWT payload (second segment)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3
        && let Ok(payload_bytes) = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            parts[1],
        )
        && let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&payload_bytes)
    {
        // Try hex_identity first (legacy tokens)
        if let Some(hex_id) = payload.get("hex_identity").and_then(|v| v.as_str()) {
            return hex_id.to_string();
        }
        // Derive identity from iss + sub claims (SpacetimeDB OIDC tokens)
        if let (Some(iss), Some(sub)) = (
            payload.get("iss").and_then(|v| v.as_str()),
            payload.get("sub").and_then(|v| v.as_str()),
        ) {
            let hex = identity_from_claims(iss, sub);
            log::info!("Derived identity from claims (iss={iss}, sub={sub}): {hex}");
            return hex;
        }
    }
    token.to_string()
}

/// Derive a SpacetimeDB identity from JWT issuer and subject claims.
/// This replicates `Identity::from_claims` from the SpacetimeDB source:
/// - blake3("{iss}|{sub}") → first 26 bytes as id_hash
/// - checksum = blake3([0xc2, 0x00] ++ id_hash)[0..4]
/// - final = [0xc2, 0x00, checksum[0..4], id_hash[0..26]] (32 bytes big-endian)
fn identity_from_claims(issuer: &str, subject: &str) -> String {
    let input = format!("{issuer}|{subject}");
    let first_hash = blake3::hash(input.as_bytes());
    let id_hash = &first_hash.as_bytes()[..26];

    // Compute checksum: blake3([0xc2, 0x00] ++ id_hash)[0..4]
    let mut checksum_input = [0u8; 28];
    checksum_input[0] = 0xc2;
    checksum_input[1] = 0x00;
    checksum_input[2..].copy_from_slice(id_hash);
    let checksum_hash = blake3::hash(&checksum_input);
    let checksum = &checksum_hash.as_bytes()[..4];

    // Build final 32-byte identity: [0xc2, 0x00, checksum(4), id_hash(26)]
    let mut final_bytes = [0u8; 32];
    final_bytes[0] = 0xc2;
    final_bytes[1] = 0x00;
    final_bytes[2..6].copy_from_slice(checksum);
    final_bytes[6..32].copy_from_slice(id_hash);

    // Hex encode (big-endian)
    final_bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Perform the full OAuth login flow via a SpacetimeDB-compatible auth host:
/// 1. Request a login token from the auth host
/// 2. Open the browser for the user to authenticate
/// 3. Poll until approved
/// 4. Exchange session token for a SpacetimeDB OIDC token
async fn perform_oauth_login(auth_host: &str) -> Result<String, String> {
    let host = auth_host.trim_end_matches('/');
    let host_opt = Some(host);

    // Step 1: Request login token
    let (browser_url, request_token) = api::oauth_request_login(host_opt).await?;

    // Step 2: Open browser
    log::info!("OAuth: opening browser to {browser_url}");
    if let Err(e) = open::that(&browser_url) {
        return Err(format!("Failed to open browser: {e}. Please open this URL manually: {browser_url}"));
    }

    // Step 3: Poll for approval
    let session_token = api::oauth_poll_approval(host_opt, &request_token).await?;

    // Step 4: Exchange for SpacetimeDB token
    let spacetimedb_token = api::oauth_exchange_token(host_opt, &session_token).await?;

    Ok(spacetimedb_token)
}
