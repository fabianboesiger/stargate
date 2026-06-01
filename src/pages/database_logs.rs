use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::{LogEntry, LogStreamMessage};
use crate::components::{DataTable, DATA_TABLE_PAGE_SIZE, Icon, IconName, PageLayout, ErrorMessage};
use crate::Route;

#[component]
pub fn DatabaseLogs(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut logs = use_signal(Vec::<LogEntry>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut page = use_signal(|| 0usize);
    let mut search = use_signal(String::new);
    let mut stream_status = use_signal(|| "Connecting...".to_string());

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    let db_id = db_identity.clone();

    use_effect(move || {
        let db_id = db_id.clone();
        spawn(async move {
            let state = app_state.read();
            let Some(api) = state.api.clone() else {
                return;
            };
            drop(state);

            let mut rx = api.subscribe_logs(&db_id, 200);
            stream_status.set("Live".to_string());
            loading.set(false);

            // Coalesce incoming log entries: collect them in a local buffer and
            // append to the reactive `logs` signal in a single batched write on a
            // fixed cadence, so a burst of entries can't flood the renderer.
            let mut pending: Vec<LogEntry> = Vec::new();
            let mut flush = tokio::time::interval(tokio::time::Duration::from_millis(250));
            flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    biased;
                    maybe_msg = rx.recv() => {
                        let Some(msg) = maybe_msg else { break; };
                        match msg {
                            LogStreamMessage::Entry(entry) => {
                                pending.push(entry);
                            }
                            LogStreamMessage::Error(e) => {
                                log::error!("Log stream error: {e}");
                                error_msg.set(Some(format!("Log stream error: {e}")));
                                stream_status.set("Error".to_string());
                                break;
                            }
                            LogStreamMessage::Disconnected => {
                                stream_status.set("Disconnected".to_string());
                                break;
                            }
                        }
                    }
                    _ = flush.tick() => {
                        if !pending.is_empty() {
                            let mut current = logs.read().clone();
                            current.append(&mut pending);
                            logs.set(current);
                        }
                    }
                }
            }

            // Flush any entries buffered since the last tick before exiting.
            if !pending.is_empty() {
                let mut current = logs.read().clone();
                current.append(&mut pending);
                logs.set(current);
            }
        });
    });

    let header_status_color = match stream_status.read().as_str() {
        "Live" => "text-emerald-400 bg-emerald-500/10",
        "Error" => "text-red-400 bg-red-500/10",
        "Disconnected" => "text-gray-500 bg-gray-500/10",
        _ => "text-yellow-400 bg-yellow-500/10",
    };

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Logs",
            title: "Logs".to_string(),
            header_extra: Some(rsx! {
                span { class: "inline-flex items-center gap-1.5 text-xs font-medium px-2.5 py-1 rounded-full {header_status_color}",
                    Icon { name: IconName::Circle, class: "w-1.5 h-1.5" }
                    "{stream_status}"
                }
            }),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col",
                if *loading.read() {
                    div { class: "text-gray-500 text-sm", "Loading..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "mb-4",
                        ErrorMessage { message: err.clone() }
                    }
                }

                if !*loading.read() {
                    {
                        let query = search.read().to_lowercase();
                        let all_logs = logs.read();
                        let filtered: Vec<&LogEntry> = if query.is_empty() {
                            all_logs.iter().collect()
                        } else {
                            all_logs
                                .iter()
                                .filter(|entry| {
                                    entry.level.to_lowercase().contains(&query)
                                        || entry.target.to_lowercase().contains(&query)
                                        || entry.message.to_lowercase().contains(&query)
                                })
                                .collect()
                        };
                        let total = filtered.len();
                        let current_page = *page.read();
                        let start = current_page * DATA_TABLE_PAGE_SIZE;
                        let page_logs: Vec<&LogEntry> = filtered
                            .into_iter()
                            .skip(start)
                            .take(DATA_TABLE_PAGE_SIZE)
                            .collect();
                        rsx! {
                            DataTable {
                                columns: vec![
                                    ("Level".into(), String::new()),
                                    ("Timestamp".into(), String::new()),
                                    ("Message".into(), String::new()),
                                ],
                                total_rows: Some(total),
                                current_page: Some(current_page),
                                on_page_change: move |new_page: usize| {
                                    page.set(new_page);
                                },
                                toolbar: rsx! {
                                    div { class: "px-5 py-3 border-b border-gray-800 flex items-center justify-between",
                                        input {
                                            class: "bg-gray-800 border border-gray-700/50 rounded-md pl-3 pr-3 py-1.5 text-xs text-gray-300 placeholder-gray-600 focus:outline-none focus:border-blue-500/50 w-56",
                                            r#type: "text",
                                            placeholder: "Filter logs...",
                                            value: "{search}",
                                            oninput: move |e| {
                                                search.set(e.value());
                                                page.set(0);
                                            },
                                        }
                                        div { class: "flex items-center gap-3",
                                            span { class: "text-xs text-gray-600", "{total} entries" }
                                        }
                                    }
                                },
                                for entry in page_logs.iter() {
                                    LogRow { entry: (*entry).clone() }
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
fn LogRow(entry: LogEntry) -> Element {
    let level_upper = entry.level.to_uppercase();
    let level_class = match level_upper.as_str() {
        "ERROR" => "text-red-400",
        "WARN" | "WARNING" => "text-yellow-400",
        "INFO" => "text-blue-400",
        "DEBUG" => "text-gray-500",
        "TRACE" => "text-gray-600",
        _ => "text-gray-400",
    };
    log::debug!("Log level: '{}' -> {}", entry.level, level_class);

    rsx! {
        tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors",
            td { class: "px-5 py-2.5 font-mono text-xs font-medium {level_class}",
                "{entry.level}"
            }
            td { class: "px-5 py-2.5 text-gray-400 font-mono text-xs", "{entry.ts}" }
            td { class: "px-5 py-2.5 text-gray-300 text-xs", "{entry.message}" }
        }
    }
}
