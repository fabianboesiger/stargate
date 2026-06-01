use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::SqlResult;
use crate::components::{ConfirmPopup, ConfirmStyle, DataTable, DATA_TABLE_PAGE_SIZE, ErrorMessage, PageLayout};
use crate::Route;

#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction {
    ExecuteQuery,
}

#[component]
pub fn Sql(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut query_text = use_signal(String::new);
    let mut result = use_signal(|| Option::<SqlResult>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut executing = use_signal(|| false);
    let mut page = use_signal(|| 0usize);
    let mut trigger_execute = use_signal(|| 0u32);
    let mut confirm_action = use_signal(|| Option::<ConfirmAction>::None);

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    let run_query = move |_: Event<MouseData>| {
        let query = query_text.peek().to_lowercase();
        let trimmed = query.trim_start();
        let is_readonly_query = trimmed.starts_with("select")
            || trimmed.starts_with("show")
            || trimmed.starts_with("explain");
        if is_readonly_query {
            trigger_execute.set(trigger_execute() + 1);
        } else if app_state.read().readonly {
            error_msg.set(Some("Cannot execute mutating queries in read-only mode".into()));
        } else {
            confirm_action.set(Some(ConfirmAction::ExecuteQuery));
        }
    };

    // Execute SQL when triggered
    let db_id_for_effect = db_identity.clone();
    use_effect(move || {
        let _trigger = *trigger_execute.read();
        if _trigger == 0 {
            return;
        }

        let query = query_text.peek().clone();
        if query.trim().is_empty() {
            error_msg.set(Some("Query cannot be empty".into()));
            return;
        }

        executing.set(true);
        error_msg.set(None);
        result.set(None);
        page.set(0);

        let db_id = db_id_for_effect.clone();
        spawn(async move {
            let state = app_state.read();
            if let Some(api) = &state.api {
                let api = api.clone();
                drop(state);

                log::info!("Executing SQL on {db_id}: {query}");
                match api.execute_sql(&db_id, &query).await {
                    Ok(res) => {
                        log::info!("SQL result: {} columns, {} rows", res.columns.len(), res.rows.len());
                        result.set(Some(res));
                        error_msg.set(None);
                    }
                    Err(e) => {
                        log::error!("SQL error: {e}");
                        error_msg.set(Some(e));
                    }
                }
            }
            executing.set(false);
        });
    });

    let current_page = *page.read();

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "SQL",
            title: "SQL".to_string(),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col gap-4",
                // Query editor
                div { class: "flex flex-col gap-3",
                    div { class: "relative",
                        textarea {
                            class: "w-full h-32 bg-gray-900 border border-gray-800 rounded-xl px-4 py-3 text-sm text-gray-200 font-mono placeholder-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500/50 resize-y",
                            placeholder: "SELECT * FROM my_table LIMIT 10;",
                            value: "{query_text}",
                            oninput: move |e| query_text.set(e.value()),
                            onkeypress: move |e| {
                                let modifier_held = if cfg!(target_os = "macos") {
                                    e.modifiers().contains(Modifiers::META)
                                } else {
                                    e.modifiers().contains(Modifiers::CONTROL)
                                };
                                if e.key() == Key::Enter && modifier_held {
                                    let query = query_text.peek().to_lowercase();
                                    let trimmed = query.trim_start();
                                    let is_readonly_query = trimmed.starts_with("select")
                                        || trimmed.starts_with("show")
                                        || trimmed.starts_with("explain");
                                    if is_readonly_query {
                                        trigger_execute.set(trigger_execute() + 1);
                                    } else if app_state.read().readonly {
                                        error_msg
                                            .set(
                                                Some("Cannot execute mutating queries in read-only mode".into()),
                                            );
                                    } else {
                                        confirm_action.set(Some(ConfirmAction::ExecuteQuery));
                                    }
                                }
                            },
                        }
                    }
                    div { class: "flex items-center justify-between",
                        span { class: "text-xs text-gray-600",
                            if cfg!(target_os = "macos") {
                                "\u{2318}+Enter to execute"
                            } else {
                                "Ctrl+Enter to execute"
                            }
                        }
                        button {
                            class: "px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: *executing.read(),
                            onclick: run_query,
                            if *executing.read() {
                                "Executing..."
                            } else {
                                "Execute"
                            }
                        }
                    }
                }

                // Error
                if let Some(err) = error_msg.read().as_ref() {
                    ErrorMessage { message: err.clone() }
                }

                // Results
                if let Some(res) = result.read().as_ref() {
                    if res.columns.is_empty() && res.rows.is_empty() {
                        div { class: "text-gray-500 text-sm bg-gray-900 border border-gray-800 rounded-xl px-5 py-4",
                            "Query executed successfully (no results returned)."
                        }
                    } else {
                        {
                            let total = res.rows.len();
                            let start = current_page * DATA_TABLE_PAGE_SIZE;
                            let page_rows: Vec<&Vec<serde_json::Value>> = res
                                .rows
                                .iter()
                                .skip(start)
                                .take(DATA_TABLE_PAGE_SIZE)
                                .collect();
                            let columns = res.columns.clone();
                            rsx! {
                                DataTable {
                                    columns: columns.iter().map(|c| (c.clone(), String::new())).collect::<Vec<_>>(),
                                    total_rows: Some(total),
                                    current_page: Some(current_page),
                                    on_page_change: move |new_page: usize| {
                                        page.set(new_page);
                                    },
                                    for row in page_rows.iter() {
                                        tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors",
                                            for cell in row.iter() {
                                                td { class: "px-5 py-2.5 text-gray-300 text-xs font-mono whitespace-nowrap max-w-xs truncate",
                                                    "{format_cell(cell)}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Confirm action popup
            if confirm_action.read().is_some() {
                ConfirmPopup {
                    title: "Confirm SQL Execution".to_string(),
                    message: "This query may modify the database. Continue?".to_string(),
                    confirm_label: "Execute".to_string(),
                    style: ConfirmStyle::Warning,
                    on_cancel: move |_| {
                        confirm_action.set(None);
                    },
                    on_confirm: move |_| {
                        confirm_action.set(None);
                        trigger_execute.set(trigger_execute() + 1);
                    },
                }
            }
        }
    }
}

fn format_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => value.to_string(),
    }
}
