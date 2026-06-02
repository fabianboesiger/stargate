use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::SqlResult;
use crate::components::{ConfirmPopup, ConfirmStyle, DataTable, DATA_TABLE_PAGE_SIZE, ErrorMessage, Icon, IconName, PageLayout};
use crate::storage::{SqlHistoryEntry, Storage};
use crate::Route;

const SQL_HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction {
    ExecuteQuery,
}

#[component]
pub fn Sql(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let storage = use_context::<Storage>();
    let mut query_text = use_signal(String::new);
    let mut result = use_signal(|| Option::<SqlResult>::None);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut executing = use_signal(|| false);
    let mut page = use_signal(|| 0usize);
    let mut trigger_execute = use_signal(|| 0u32);
    let mut confirm_action = use_signal(|| Option::<ConfirmAction>::None);
    let mut show_history = use_signal(|| false);
    let mut history = use_signal({
        let storage = storage.clone();
        let db = db_identity.clone();
        move || storage.list_history(&db, SQL_HISTORY_LIMIT)
    });
    let mut pending_clear_history = use_signal(|| false);

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
    let storage_for_effect = storage.clone();
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
        let storage = storage_for_effect.clone();
        spawn(async move {
            let state = app_state.read();
            if let Some(api) = &state.api {
                let api = api.clone();
                drop(state);

                log::info!("Executing SQL on {db_id}: {query}");
                let success = match api.execute_sql(&db_id, &query).await {
                    Ok(res) => {
                        log::info!("SQL result: {} columns, {} rows", res.columns.len(), res.rows.len());
                        result.set(Some(res));
                        error_msg.set(None);
                        true
                    }
                    Err(e) => {
                        log::error!("SQL error: {e}");
                        error_msg.set(Some(e));
                        false
                    }
                };
                storage.add_history(&db_id, &query, success);
                history.set(storage.list_history(&db_id, SQL_HISTORY_LIMIT));
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
                        div { class: "flex items-center gap-2",
                            button {
                                class: "flex items-center gap-1.5 px-3 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium rounded-lg transition-colors",
                                onclick: move |_| {
                                    let next = !*show_history.read();
                                    show_history.set(next);
                                },
                                Icon {
                                    name: IconName::Clock,
                                    class: "w-3.5 h-3.5",
                                }
                                "History"
                                if !history.read().is_empty() {
                                    span { class: "text-xs text-gray-500", "({history.read().len()})" }
                                }
                            }
                            button {
                                class: "px-4 py-2 bg-blue-600 hover:bg-blue-500 text-gray-950 text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
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
                }

                // History panel
                if *show_history.read() {
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl overflow-hidden",
                        div { class: "flex items-center justify-between px-4 py-2.5 border-b border-gray-800",
                            span { class: "text-xs font-medium text-gray-400 uppercase tracking-wide",
                                "Query history"
                            }
                            if !history.read().is_empty() {
                                button {
                                    class: "flex items-center gap-1.5 text-xs text-gray-500 hover:text-red-600 transition-colors",
                                    onclick: move |_| pending_clear_history.set(true),
                                    Icon {
                                        name: IconName::Trash,
                                        class: "w-3.5 h-3.5",
                                    }
                                    "Clear"
                                }
                            }
                        }
                        if history.read().is_empty() {
                            div { class: "px-4 py-6 text-sm text-gray-600 text-center",
                                "No queries executed yet."
                            }
                        } else {
                            div { class: "max-h-64 overflow-y-auto divide-y divide-gray-800/50",
                                for entry in history.read().iter() {
                                    SqlHistoryRow {
                                        key: "{entry.id}",
                                        entry: entry.clone(),
                                        on_select: move |q: String| query_text.set(q),
                                    }
                                }
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

            // Confirm clearing history
            if *pending_clear_history.read() {
                ConfirmPopup {
                    title: "Clear query history".to_string(),
                    message: "Remove all saved queries for this database from this device?".to_string(),
                    confirm_label: "Clear".to_string(),
                    style: ConfirmStyle::Danger,
                    on_cancel: move |_| pending_clear_history.set(false),
                    on_confirm: move |_| {
                        storage.clear_history(&db_identity);
                        history.set(storage.list_history(&db_identity, SQL_HISTORY_LIMIT));
                        pending_clear_history.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn SqlHistoryRow(entry: SqlHistoryEntry, on_select: EventHandler<String>) -> Element {
    let query = entry.query.clone();
    let preview: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
    let time = chrono::DateTime::from_timestamp(entry.executed_at, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();

    rsx! {
        button {
            class: "w-full text-left px-4 py-2.5 hover:bg-gray-800/40 transition-colors flex items-start gap-3",
            onclick: move |_| on_select.call(query.clone()),
            span { class: if entry.success { "mt-1.5 w-1.5 h-1.5 rounded-full bg-green-500 shrink-0" } else { "mt-1.5 w-1.5 h-1.5 rounded-full bg-red-500 shrink-0" } }
            div { class: "min-w-0 flex-1",
                div { class: "text-xs text-gray-300 font-mono truncate", "{preview}" }
                div { class: "text-[11px] text-gray-600 mt-0.5", "{time}" }
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
