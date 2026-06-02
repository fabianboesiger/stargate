use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::TableSchema;
use crate::components::{DataTable, DATA_TABLE_PAGE_SIZE, PageLayout, ErrorMessage};
use crate::Route;

#[component]
pub fn DatabaseTables(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut tables = use_signal(Vec::<TableSchema>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut search = use_signal(String::new);
    let mut page = use_signal(|| 0usize);

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
            if let Some(api) = &state.api {
                let api = api.clone();
                drop(state);

                match api.get_schema(&db_id).await {
                    Ok(schema) => {
                        tables.set(schema.tables);
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Failed to load schema: {e}")));
                    }
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Tables",
            title: "Tables".to_string(),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col",
                if *loading.read() {
                    div { class: "text-gray-500 text-sm", "Loading..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "mb-4",
                        ErrorMessage { message: err.clone() }
                    }
                }

                if !*loading.read() && !tables.read().is_empty() {
                    {
                        let query = search.read().to_lowercase();
                        let all_tables = tables.read();
                        let filtered: Vec<&TableSchema> = if query.is_empty() {
                            all_tables.iter().collect()
                        } else {
                            all_tables
                                .iter()
                                .filter(|t| { t.name.to_lowercase().contains(&query) })
                                .collect()
                        };
                        let total = filtered.len();
                        let current_page = *page.read();
                        let start = current_page * DATA_TABLE_PAGE_SIZE;
                        let page_tables: Vec<&TableSchema> = filtered
                            .into_iter()
                            .skip(start)
                            .take(DATA_TABLE_PAGE_SIZE)
                            .collect();
                        rsx! {
                            DataTable {
                                columns: vec![
                                    ("Table Name".into(), String::new()),
                                    ("Type".into(), String::new()),
                                    ("Access".into(), String::new()),
                                    ("Indexes".into(), String::new()),
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
                                            placeholder: "Filter tables...",
                                            value: "{search}",
                                            oninput: move |e| {
                                                search.set(e.value());
                                                page.set(0);
                                            },
                                        }
                                        span { class: "text-xs text-gray-600", "{total} tables" }
                                    }
                                },
                                for table in page_tables.iter() {
                                    TableRow { table: (*table).clone(), db_identity: db_identity.clone() }
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
fn TableRow(table: TableSchema, db_identity: String) -> Element {
    let access = if table.table_access.get("Public").is_some() {
        "Public"
    } else {
        "Private"
    };
    let table_type = if table.table_type.get("User").is_some() {
        "User"
    } else {
        "System"
    };
    let index_count = table.indexes.len();
    let table_name = table.name.clone();

    let type_classes = if table_type == "User" {
        "text-xs px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400 font-medium"
    } else {
        "text-xs px-2 py-0.5 rounded-full bg-gray-700/50 text-gray-400 font-medium"
    };
    let access_classes = if access == "Public" {
        "text-xs px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-600 font-medium"
    } else {
        "text-xs px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 font-medium"
    };
    let name_color = if table_type == "User" {
        "font-medium text-gray-50"
    } else {
        "font-medium text-gray-400"
    };

    rsx! {
        tr {
            class: "border-b border-gray-800/50 hover:bg-gray-800/50 cursor-pointer transition-all duration-150",
            onclick: move |_| {
                navigator()
                    .push(Route::TableData {
                        db_identity: db_identity.clone(),
                        table_name: table_name.clone(),
                    });
            },
            td { class: "px-5 py-3.5",
                span { class: "{name_color}", "{table.name}" }
            }
            td { class: "px-5 py-3.5",
                span { class: "{type_classes}", "{table_type}" }
            }
            td { class: "px-5 py-3.5",
                span { class: "{access_classes}", "{access}" }
            }
            td { class: "px-5 py-3.5 text-gray-400", "{index_count}" }
        }
    }
}
