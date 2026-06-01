use dioxus::prelude::*;
use crate::state::AppState;
use crate::components::{ErrorMessage, PageLayout};
use crate::Route;

#[component]
pub fn Info(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut db_name = use_signal(String::new);
    let mut table_count = use_signal(|| 0usize);
    let mut reducer_count = use_signal(|| 0usize);
    let mut connected_clients = use_signal(|| Option::<u64>::None);

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    // Fetch info on mount
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let db_identity = db_identity.clone();
            spawn(async move {
                let state = app_state.read();
                if let Some(api) = &state.api {
                    let api = api.clone();
                    drop(state);

                    // Get database name
                    if let Ok(names) = api.get_database_names(&db_identity).await
                        && let Some(name) = names.into_iter().next()
                    {
                        db_name.set(name);
                    }

                    // Get schema for table/reducer counts
                    match api.get_schema(&db_identity).await {
                        Ok(schema) => {
                            table_count.set(schema.tables.len());
                            reducer_count.set(schema.reducers.len());

                            // Try to get connected clients count
                            if let Ok(result) = api.execute_sql(&db_identity, "SELECT count(*) FROM st_client").await
                                && let Some(v) = result.rows.first().and_then(|row| row.first())
                            {
                                let count = v.as_u64()
                                    .or_else(|| v.as_i64().map(|n| n as u64))
                                    .or_else(|| v.as_f64().map(|n| n as u64))
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                                    .unwrap_or(0);
                                connected_clients.set(Some(count));
                            }

                            error_msg.set(None);
                        }
                        Err(e) => {
                            log::error!("Failed to fetch schema for info: {e}");
                            error_msg.set(Some(format!("Failed to load info: {e}")));
                        }
                    }
                }
                loading.set(false);
            });
        }
    });

    let server_url = app_state.read().server_url.clone();
    let identity = app_state.read().identity.clone();

    let display_name = {
        let name = db_name.read();
        if name.is_empty() {
            format!("{}...", &db_identity[..16.min(db_identity.len())])
        } else {
            name.clone()
        }
    };

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Info",
            title: "Info".to_string(),
            div { class: "px-8 pb-8 flex-1 min-h-0 overflow-y-auto flex flex-col gap-6",
                if *loading.read() {
                    div { class: "text-gray-500 text-sm", "Loading..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    ErrorMessage { message: err.clone() }
                }

                if !*loading.read() {
                    // Connection info
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl overflow-hidden",
                        div { class: "divide-y divide-gray-800/50",
                            InfoRow {
                                label: "Server URL",
                                value: server_url.clone(),
                            }
                            InfoRow { label: "Identity", value: identity.clone() }
                            InfoRow {
                                label: "Database",
                                value: display_name.clone(),
                            }
                            InfoRow {
                                label: "Tables",
                                value: format!("{}", *table_count.read()),
                            }
                            InfoRow {
                                label: "Reducers",
                                value: format!("{}", *reducer_count.read()),
                            }
                            if let Some(clients) = *connected_clients.read() {
                                InfoRow {
                                    label: "Connected Clients",
                                    value: format!("{clients}"),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn InfoRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "px-5 py-3 flex items-center justify-between gap-4",
            span { class: "text-sm text-gray-500", "{label}" }
            span { class: "text-sm text-gray-300 font-mono truncate max-w-[70%] text-right",
                "{value}"
            }
        }
    }
}
