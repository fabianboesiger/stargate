use dioxus::prelude::*;
use crate::state::AppState;
use crate::components::{ConfirmPopup, ConfirmStyle, DataTable, DATA_TABLE_PAGE_SIZE, Icon, IconName, PageLayout, ErrorMessage};
use crate::ws::{self, TableUpdate};
use crate::Route;

#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction {
    EditCell,
    DeleteRow,
    InsertRow,
}

#[component]
pub fn TableData(db_identity: String, table_name: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut rows = use_signal(Vec::<serde_json::Value>::new);
    let mut columns = use_signal(Vec::<String>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut live_status = use_signal(|| "Connecting...".to_string());
    let mut page = use_signal(|| 0usize);
    let mut search_query = use_signal(String::new);
    let mut search_input = use_signal(String::new);
    let mut _db_name = use_signal(String::new);
    let mut editing_cell = use_signal(|| Option::<(usize, String)>::None);
    let mut edit_value = use_signal(String::new);
    let mut edit_saving = use_signal(|| false);
    let mut edit_error = use_signal(|| Option::<String>::None);
    let mut deleting_row = use_signal(|| Option::<usize>::None);
    let mut delete_saving = use_signal(|| false);
    let mut delete_error = use_signal(|| Option::<String>::None);
    let mut inserting = use_signal(|| false);
    let mut insert_values = use_signal(Vec::<(String, String)>::new);
    let mut insert_saving = use_signal(|| false);
    let mut insert_error = use_signal(|| Option::<String>::None);
    let mut confirm_action = use_signal(|| Option::<ConfirmAction>::None);
    let mut trigger_edit = use_signal(|| 0u32);
    let mut trigger_delete = use_signal(|| 0u32);
    let mut trigger_insert = use_signal(|| 0u32);

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    let db_id = db_identity.clone();
    let tbl_name = table_name.clone();

    // Fetch schema for column names + start WebSocket subscription
    use_effect(move || {
        let db_id = db_id.clone();
        let tbl_name = tbl_name.clone();
        spawn(async move {
            let state = app_state.read();
            let Some(api) = state.api.clone() else {
                return;
            };
            let base_url = state.server_url.clone();
            let token = if state.token.is_empty() { None } else { Some(state.token.clone()) };
            drop(state);

            // Fetch database name
            if let Ok(names) = api.get_database_names(&db_id).await
                && !names.is_empty()
            {
                _db_name.set(names.join(", "));
            }

            // Fetch column names via a quick SQL query
            let col_query = format!("SELECT * FROM {} LIMIT 1", tbl_name);
            if let Ok(result) = api.execute_sql(&db_id, &col_query).await
                && !result.columns.is_empty()
            {
                columns.set(result.columns);
            }

            // Start WebSocket subscription
            let mut rx = ws::subscribe_to_table(
                &base_url,
                &db_id,
                &tbl_name,
                token.as_deref(),
            );

            live_status.set("Connected".to_string());

            // Coalesce high-frequency updates: apply them to a local buffer and
            // only push to the reactive `rows` signal on a fixed cadence, so the
            // renderer is never flooded regardless of update volume.
            let mut buffer: Vec<serde_json::Value> = Vec::new();
            let mut dirty = false;
            let mut initialized = false;
            let mut flush = tokio::time::interval(tokio::time::Duration::from_millis(250));
            flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    biased;
                    maybe_update = rx.recv() => {
                        let Some(update) = maybe_update else { break; };
                        match update {
                            TableUpdate::InitialRows { rows: initial_rows, .. } => {
                                log::info!("Received {} initial rows", initial_rows.len());
                                buffer = initial_rows;
                                rows.set(buffer.clone());
                                loading.set(false);
                                live_status.set("Live".to_string());
                                initialized = true;
                                dirty = false;
                            }
                            TableUpdate::Insert { rows: new_rows, .. } => {
                                buffer.extend(new_rows);
                                dirty = true;
                            }
                            TableUpdate::Delete { rows: del_rows, .. } => {
                                buffer.retain(|row| !del_rows.contains(row));
                                dirty = true;
                            }
                            TableUpdate::Error(e) => {
                                log::error!("Subscription error: {e}");
                                error_msg.set(Some(e));
                                live_status.set("Disconnected".to_string());
                                loading.set(false);
                                break;
                            }
                        }
                    }
                    _ = flush.tick() => {
                        if dirty && initialized {
                            rows.set(buffer.clone());
                            dirty = false;
                        }
                    }
                }
            }

            // Push any remaining buffered changes before tearing down.
            if dirty && initialized {
                rows.set(buffer.clone());
            }

            // If loop exits without error, connection was closed
            if live_status.read().as_str() != "Disconnected" {
                log::warn!("Table data WebSocket connection ended");
                live_status.set("Disconnected".to_string());
            }
        });
    });

    // Execute cell edit when confirmed
    use_effect({
        let table_name = table_name.clone();
        let db_identity = db_identity.clone();
        move || {
            let _trigger = *trigger_edit.read();
            if _trigger == 0 {
                return;
            }
            let table_name = table_name.clone();
            let db_identity = db_identity.clone();
            let Some((row_idx, ref col)) = *editing_cell.read() else {
                return;
            };
            let col = col.clone();
            spawn(async move {
                edit_saving.set(true);
                edit_error.set(None);

                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    edit_saving.set(false);
                    return;
                };
                drop(state);

                let new_val = edit_value.read().clone();
                let all_rows = rows.read();
                let Some(row) = all_rows.get(row_idx) else {
                    edit_error.set(Some("Row no longer exists".into()));
                    edit_saving.set(false);
                    return;
                };

                let set_clause = format_sql_value(&col, &new_val);
                let where_clause = build_where_clause(row);

                if where_clause.is_empty() {
                    edit_error.set(Some("Cannot identify row for update".into()));
                    edit_saving.set(false);
                    return;
                }

                let count_sql = format!(
                    "SELECT count(*) AS cnt FROM {} WHERE {}",
                    table_name, where_clause,
                );
                drop(all_rows);
                match api.execute_sql(&db_identity, &count_sql).await {
                    Ok(result) => {
                        let count = result
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .and_then(|v| {
                                v.as_u64()
                                    .or_else(|| v.as_i64().map(|n| n as u64))
                                    .or_else(|| v.as_f64().map(|n| n as u64))
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                            })
                            .unwrap_or(0);
                        if count != 1 {
                            edit_error.set(Some(format!(
                                "Expected 1 matching row, found {count}. Update aborted."
                            )));
                            edit_saving.set(false);
                            return;
                        }
                    }
                    Err(e) => {
                        log::error!("Row count check failed: {e}");
                        edit_error.set(Some(format!("Failed to verify row: {e}")));
                        edit_saving.set(false);
                        return;
                    }
                }

                let sql = format!(
                    "UPDATE {} SET {} WHERE {}",
                    table_name, set_clause, where_clause,
                );
                log::info!("Executing cell update: {sql}");
                match api.execute_sql(&db_identity, &sql).await {
                    Ok(_) => {
                        editing_cell.set(None);
                    }
                    Err(e) => {
                        log::error!("Cell update failed: {e}");
                        edit_error.set(Some(e));
                    }
                }
                edit_saving.set(false);
            });
        }
    });

    // Execute row delete when confirmed
    use_effect({
        let table_name = table_name.clone();
        let db_identity = db_identity.clone();
        move || {
            let _trigger = *trigger_delete.read();
            if _trigger == 0 {
                return;
            }
            let table_name = table_name.clone();
            let db_identity = db_identity.clone();
            let Some(row_idx) = *deleting_row.read() else {
                return;
            };
            spawn(async move {
                delete_saving.set(true);
                delete_error.set(None);

                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    delete_saving.set(false);
                    return;
                };
                drop(state);

                let all_rows = rows.read();
                let Some(row) = all_rows.get(row_idx) else {
                    delete_error.set(Some("Row no longer exists".into()));
                    delete_saving.set(false);
                    return;
                };

                let where_clause = build_where_clause(row);
                drop(all_rows);

                if where_clause.is_empty() {
                    delete_error.set(Some("Cannot identify row for deletion".into()));
                    delete_saving.set(false);
                    return;
                }

                let count_sql = format!(
                    "SELECT count(*) AS cnt FROM {} WHERE {}",
                    table_name, where_clause,
                );
                match api.execute_sql(&db_identity, &count_sql).await {
                    Ok(result) => {
                        let count = result
                            .rows
                            .first()
                            .and_then(|r| r.first())
                            .and_then(|v| {
                                v.as_u64()
                                    .or_else(|| v.as_i64().map(|n| n as u64))
                                    .or_else(|| v.as_f64().map(|n| n as u64))
                                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                            })
                            .unwrap_or(0);
                        if count != 1 {
                            delete_error.set(Some(format!(
                                "Expected 1 matching row, found {count}. Delete aborted."
                            )));
                            delete_saving.set(false);
                            return;
                        }
                    }
                    Err(e) => {
                        log::error!("Row count check failed: {e}");
                        delete_error.set(Some(format!("Failed to verify row: {e}")));
                        delete_saving.set(false);
                        return;
                    }
                }

                let sql = format!(
                    "DELETE FROM {} WHERE {}",
                    table_name, where_clause,
                );
                log::info!("Executing row delete: {sql}");
                match api.execute_sql(&db_identity, &sql).await {
                    Ok(_) => {
                        deleting_row.set(None);
                    }
                    Err(e) => {
                        log::error!("Row delete failed: {e}");
                        delete_error.set(Some(e));
                    }
                }
                delete_saving.set(false);
            });
        }
    });

    // Execute row insert when confirmed
    use_effect({
        let table_name = table_name.clone();
        let db_identity = db_identity.clone();
        move || {
            let _trigger = *trigger_insert.read();
            if _trigger == 0 {
                return;
            }
            let table_name = table_name.clone();
            let db_identity = db_identity.clone();
            spawn(async move {
                insert_saving.set(true);
                insert_error.set(None);

                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    insert_saving.set(false);
                    return;
                };
                drop(state);

                let values = insert_values.read().clone();
                let col_list: Vec<&str> = values
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(c, _)| c.as_str())
                    .collect();
                let val_list: Vec<String> = values
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(_, v)| format_insert_value(v))
                    .collect();

                if col_list.is_empty() {
                    insert_error.set(Some("At least one value is required".into()));
                    insert_saving.set(false);
                    return;
                }

                let sql = format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    table_name,
                    col_list.join(", "),
                    val_list.join(", "),
                );
                log::info!("Executing row insert: {sql}");
                match api.execute_sql(&db_identity, &sql).await {
                    Ok(_) => {
                        inserting.set(false);
                    }
                    Err(e) => {
                        log::error!("Row insert failed: {e}");
                        insert_error.set(Some(e));
                    }
                }
                insert_saving.set(false);
            });
        }
    });

    let current_page = *page.read();
    let current_search = search_query.read().clone();

    // Client-side filtering and pagination
    let all_rows = rows.read();
    let filtered_rows: Vec<&serde_json::Value> = if current_search.is_empty() {
        all_rows.iter().collect()
    } else {
        all_rows
            .iter()
            .filter(|row| {
                let row_str = row.to_string().to_lowercase();
                row_str.contains(&current_search.to_lowercase())
            })
            .collect()
    };

    let total_rows = filtered_rows.len();
    let start = current_page * DATA_TABLE_PAGE_SIZE;
    let page_rows: Vec<&serde_json::Value> = filtered_rows
        .into_iter()
        .skip(start)
        .take(DATA_TABLE_PAGE_SIZE)
        .collect();

    let col_names = columns.read().clone();

    let status_color = match live_status.read().as_str() {
        "Live" => "text-emerald-600 bg-emerald-500/10",
        "Connected" => "text-blue-400 bg-blue-500/10",
        "Disconnected" => "text-red-600 bg-red-500/10",
        _ => "text-yellow-600 bg-yellow-500/10",
    };

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Tables",
            title: table_name.to_string(),
            header_extra: Some(rsx! {
                span { class: "inline-flex items-center gap-1.5 text-xs font-medium px-2.5 py-1 rounded-full {status_color}",
                    Icon { name: IconName::Circle, class: "w-1.5 h-1.5" }
                    "{live_status}"
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
                    DataTable {
                        columns: col_names.iter().map(|c| (c.clone(), String::new())).collect::<Vec<_>>(),
                        total_rows: Some(total_rows),
                        current_page: Some(current_page),
                        on_page_change: move |new_page: usize| {
                            page.set(new_page);
                        },
                        toolbar: rsx! {
                            div { class: "px-5 py-3 border-b border-gray-800 flex items-center justify-between",
                                div { class: "relative",
                                    input {
                                        class: "bg-gray-800 border border-gray-700/50 rounded-md pl-3 pr-3 py-1.5 text-xs text-gray-300 placeholder-gray-600 focus:outline-none focus:border-blue-500/50 w-56",
                                        r#type: "text",
                                        placeholder: "Filter rows...",
                                        value: "{search_input}",
                                        oninput: move |e| {
                                            search_input.set(e.value());
                                        },
                                        onkeypress: move |e| {
                                            if e.key() == Key::Enter {
                                                search_query.set(search_input.read().clone());
                                                page.set(0);
                                            }
                                        },
                                    }
                                }
                                div { class: "flex items-center gap-3",
                                    button {
                                        class: "flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700/50 rounded-md transition-colors",
                                        onclick: move |_| {
                                            let cols = columns.read().clone();
                                            insert_values.set(cols.iter().map(|c| (c.clone(), String::new())).collect());
                                            insert_error.set(None);
                                            inserting.set(true);
                                        },
                                        Icon { name: IconName::Plus, class: "w-3.5 h-3.5" }
                                        "Insert"
                                    }
                                    button {
                                        class: "flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700/50 rounded-md transition-colors",
                                        onclick: {
                                            let table_name = table_name.clone();
                                            move |_| {
                                                let cols = columns.read().clone();
                                                let all = rows.read().clone();
                                                let query = search_query.read().to_lowercase();
                                                let filtered: Vec<&serde_json::Value> = if query.is_empty() {
                                                    all.iter().collect()
                                                } else {
                                                    all.iter()
                                                        .filter(|row| row.to_string().to_lowercase().contains(&query))
                                                        .collect()
                                                };
                                                let csv = build_csv(&cols, &filtered);
                                                let file_name = format!("{}.csv", table_name);
                                                spawn(async move {
                                                    export_csv(&file_name, &csv).await;
                                                });
                                            }
                                        },
                                        Icon { name: IconName::Download, class: "w-3.5 h-3.5" }
                                        "Export"
                                    }
                                    button {
                                        class: "text-xs text-gray-500 hover:text-gray-300 px-2 py-1.5 rounded-md hover:bg-gray-800 transition-colors",
                                        onclick: move |_| {
                                            search_query.set(search_input.read().clone());
                                            page.set(0);
                                        },
                                        "Apply"
                                    }
                                    span { class: "text-xs text-gray-600", "{total_rows} rows" }
                                }
                            }
                        },
                        for (row_idx , row) in page_rows.iter().enumerate() {
                            tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors group/row",
                                for col in col_names.iter() {
                                    td { class: "px-4 py-2.5 text-gray-300 text-xs font-mono whitespace-nowrap max-w-[200px] overflow-hidden group relative",
                                        span { class: "block truncate pr-5",
                                            "{format_cell(row.get(col.as_str()).unwrap_or(&serde_json::Value::Null))}"
                                        }
                                        {
                                            let col_name = col.clone();
                                            let absolute_idx = start + row_idx;
                                            rsx! {
                                                button {
                                                    class: "absolute right-1 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 p-1 rounded bg-gray-700/80 hover:bg-gray-600 text-gray-400 hover:text-gray-200 transition-all",
                                                    onclick: move |_| {
                                                        let all = rows.read();
                                                        if let Some(r) = all.get(absolute_idx) {
                                                            let val = r.get(col_name.as_str()).unwrap_or(&serde_json::Value::Null);
                                                            edit_value.set(format_cell(val));
                                                            editing_cell.set(Some((absolute_idx, col_name.clone())));
                                                            edit_error.set(None);
                                                        }
                                                    },
                                                    Icon { name: IconName::Pencil, class: "w-3 h-3" }
                                                }
                                            }
                                        }
                                    }
                                }
                                td { class: "px-2 py-2.5 whitespace-nowrap",
                                    {
                                        let absolute_idx = start + row_idx;
                                        rsx! {
                                            button {
                                                class: "opacity-0 group-hover/row:opacity-100 p-1 rounded bg-gray-700/80 hover:bg-red-600/80 text-gray-400 hover:text-red-200 transition-all",
                                                onclick: move |_| {
                                                    delete_error.set(None);
                                                    deleting_row.set(Some(absolute_idx));
                                                },
                                                Icon { name: IconName::Trash, class: "w-3 h-3" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Cell edit modal
            if let Some((_row_idx, col_name)) = editing_cell.read().clone() {
                div {
                    class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50",
                    onclick: move |_| {
                        editing_cell.set(None);
                    },
                    div {
                        class: "bg-gray-900 border border-gray-700 rounded-xl p-6 w-[480px] max-w-[90vw] shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        h3 { class: "text-sm font-medium text-gray-300 mb-1", "Edit Cell" }
                        p { class: "text-xs text-gray-500 mb-4 font-mono", "{col_name}" }

                        textarea {
                            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-gray-200 font-mono focus:outline-none focus:border-blue-500/50 resize-y min-h-[80px]",
                            value: "{edit_value}",
                            oninput: move |e| edit_value.set(e.value()),
                        }

                        if let Some(err) = edit_error.read().as_ref() {
                            p { class: "text-xs text-red-600 mt-2", "{err}" }
                        }

                        div { class: "flex justify-end gap-2 mt-4",
                            button {
                                class: "px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md transition-colors",
                                onclick: move |_| {
                                    editing_cell.set(None);
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-3 py-1.5 text-xs text-gray-950 bg-blue-600 hover:bg-blue-500 rounded-md transition-colors disabled:opacity-50",
                                disabled: *edit_saving.read() || app_state.read().readonly,
                                onclick: move |_| {
                                    confirm_action.set(Some(ConfirmAction::EditCell));
                                },
                                if *edit_saving.read() {
                                    "Saving..."
                                } else {
                                    "Save"
                                }
                            }
                        }
                    }
                }
            }

            // Delete confirmation modal
            if let Some(_row_idx) = *deleting_row.read() {
                div {
                    class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50",
                    onclick: move |_| {
                        deleting_row.set(None);
                    },
                    div {
                        class: "bg-gray-900 border border-gray-700 rounded-xl p-6 w-[420px] max-w-[90vw] shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        h3 { class: "text-sm font-medium text-gray-300 mb-1", "Delete Row" }
                        p { class: "text-xs text-gray-500 mb-4",
                            "Are you sure you want to delete this row? This action cannot be undone."
                        }

                        if let Some(err) = delete_error.read().as_ref() {
                            p { class: "text-xs text-red-600 mb-3", "{err}" }
                        }

                        div { class: "flex justify-end gap-2",
                            button {
                                class: "px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md transition-colors",
                                onclick: move |_| {
                                    deleting_row.set(None);
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-3 py-1.5 text-xs text-white bg-red-600 hover:bg-red-500 rounded-md transition-colors disabled:opacity-50",
                                disabled: *delete_saving.read() || app_state.read().readonly,
                                onclick: move |_| {
                                    confirm_action.set(Some(ConfirmAction::DeleteRow));
                                },
                                if *delete_saving.read() {
                                    "Deleting..."
                                } else {
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }

            // Insert row modal
            if *inserting.read() {
                div {
                    class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50",
                    onclick: move |_| {
                        inserting.set(false);
                    },
                    div {
                        class: "bg-gray-900 border border-gray-700 rounded-xl p-6 w-[520px] max-w-[90vw] max-h-[80vh] shadow-2xl flex flex-col",
                        onclick: move |e| e.stop_propagation(),
                        h3 { class: "text-sm font-medium text-gray-300 mb-1", "Insert Row" }
                        p { class: "text-xs text-gray-500 mb-4",
                            "Enter values for each column. Leave empty for NULL."
                        }

                        div { class: "flex-1 overflow-y-auto space-y-3 mb-4",
                            for (idx , (col_name , _val)) in insert_values.read().iter().enumerate() {
                                div { class: "flex flex-col gap-1",
                                    label { class: "text-xs text-gray-400 font-mono",
                                        "{col_name}"
                                    }
                                    {
                                        rsx! {
                                            input {
                                                class: "w-full bg-gray-800 border border-gray-700 rounded-md px-3 py-1.5 text-sm text-gray-200 font-mono focus:outline-none focus:border-blue-500/50",
                                                r#type: "text",
                                                placeholder: "NULL",
                                                value: "{insert_values.read()[idx].1}",
                                                oninput: move |e| {
                                                    let mut vals = insert_values.read().clone();
                                                    vals[idx].1 = e.value();
                                                    insert_values.set(vals);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(err) = insert_error.read().as_ref() {
                            p { class: "text-xs text-red-600 mb-3", "{err}" }
                        }

                        div { class: "flex justify-end gap-2",
                            button {
                                class: "px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md transition-colors",
                                onclick: move |_| {
                                    inserting.set(false);
                                },
                                "Cancel"
                            }
                            button {
                                class: "px-3 py-1.5 text-xs text-gray-950 bg-blue-600 hover:bg-blue-500 rounded-md transition-colors disabled:opacity-50",
                                disabled: *insert_saving.read() || app_state.read().readonly,
                                onclick: move |_| {
                                    confirm_action.set(Some(ConfirmAction::InsertRow));
                                },
                                if *insert_saving.read() {
                                    "Inserting..."
                                } else {
                                    "Insert"
                                }
                            }
                        }
                    }
                }
            }

            // Confirm action popup
            if let Some(ref action) = *confirm_action.read() {
                {
                    let (title, message, label, style) = match action {
                        ConfirmAction::EditCell => {
                            (
                                "Confirm Edit",
                                "This will modify data in the database. Continue?",
                                "Save",
                                ConfirmStyle::Warning,
                            )
                        }
                        ConfirmAction::DeleteRow => {
                            (
                                "Confirm Delete",
                                "This will permanently delete a row from the database. Continue?",
                                "Delete",
                                ConfirmStyle::Danger,
                            )
                        }
                        ConfirmAction::InsertRow => {
                            (
                                "Confirm Insert",
                                "This will insert a new row into the database. Continue?",
                                "Insert",
                                ConfirmStyle::Warning,
                            )
                        }
                    };
                    rsx! {
                        ConfirmPopup {
                            title: title.to_string(),
                            message: message.to_string(),
                            confirm_label: label.to_string(),
                            style,
                            on_cancel: move |_| {
                                confirm_action.set(None);
                            },
                            on_confirm: move |_| {
                                let action = confirm_action.read().clone();
                                confirm_action.set(None);
                                match action {
                                    Some(ConfirmAction::EditCell) => {
                                        trigger_edit.set(trigger_edit() + 1);
                                    }
                                    Some(ConfirmAction::DeleteRow) => {
                                        trigger_delete.set(trigger_delete() + 1);
                                    }
                                    Some(ConfirmAction::InsertRow) => {
                                        trigger_insert.set(trigger_insert() + 1);
                                    }
                                    None => {}
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

fn format_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Object(map) => {
            // Handle SpacetimeDB special types
            if let Some(id) = map.get("__identity__") {
                return id.as_str().unwrap_or("").to_string();
            }
            if let Some(ts) = map.get("__timestamp_micros_since_unix_epoch__") {
                if let Some(micros) = ts.as_i64() {
                    let secs = micros / 1_000_000;
                    let dt = chrono_format_timestamp(secs);
                    return dt;
                }
                return ts.to_string();
            }
            // Generic object
            serde_json::to_string(value).unwrap_or_default()
        }
        serde_json::Value::Array(arr) => {
            arr.iter()
                .map(format_cell)
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

fn chrono_format_timestamp(epoch_secs: i64) -> String {
    // Simple UTC timestamp formatting without chrono dependency
    let secs_per_day: i64 = 86400;
    let days = epoch_secs / secs_per_day;
    let time_secs = (epoch_secs % secs_per_day) as u32;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since Unix epoch to date (simplified)
    let mut y = 1970i64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }
    let months_days: &[i64] = if is_leap_year(y) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u32;
    for &md in months_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    let d = remaining_days + 1;
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn format_sql_value(column: &str, value: &str) -> String {
    if value.eq_ignore_ascii_case("NULL") {
        format!("{column} = NULL")
    } else if value.parse::<f64>().is_ok() || value == "true" || value == "false" {
        format!("{column} = {value}")
    } else {
        let escaped = value.replace('\'', "''");
        format!("{column} = '{escaped}'")
    }
}

fn build_where_clause(row: &serde_json::Value) -> String {
    let Some(obj) = row.as_object() else {
        return String::new();
    };
    let conditions: Vec<String> = obj
        .iter()
        .filter_map(|(key, val)| {
            let condition = match val {
                serde_json::Value::Null => format!("{key} IS NULL"),
                serde_json::Value::String(s) => {
                    let escaped = s.replace('\'', "''");
                    format!("{key} = '{escaped}'")
                }
                serde_json::Value::Number(n) => format!("{key} = {n}"),
                serde_json::Value::Bool(b) => format!("{key} = {b}"),
                serde_json::Value::Object(map) => {
                    if let Some(id) = map.get("__identity__").and_then(|v| v.as_str()) {
                        format!("{key} = x'{id}'")
                    } else if let Some(ts) = map.get("__timestamp_micros_since_unix_epoch__").and_then(|v| v.as_i64()) {
                        format!("{key} = {ts}")
                    } else {
                        return None;
                    }
                }
                serde_json::Value::Array(_) => return None,
            };
            Some(condition)
        })
        .collect();
    conditions.join(" AND ")
}

fn format_insert_value(value: &str) -> String {
    if value.eq_ignore_ascii_case("NULL") {
        "NULL".to_string()
    } else if value == "true" || value == "false" || value.parse::<f64>().is_ok() {
        value.to_string()
    } else {
        let escaped = value.replace('\'', "''");
        format!("'{escaped}'")
    }
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

fn build_csv(columns: &[String], rows: &[&serde_json::Value]) -> String {
    let mut out = String::new();

    // Header row
    let header: Vec<String> = columns.iter().map(|c| escape_csv_field(c)).collect();
    out.push_str(&header.join(","));
    out.push('\n');

    // Data rows
    for row in rows {
        let fields: Vec<String> = columns
            .iter()
            .map(|col| {
                let val = row.get(col.as_str()).unwrap_or(&serde_json::Value::Null);
                escape_csv_field(&format_cell(val))
            })
            .collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }

    out
}

async fn export_csv(file_name: &str, content: &str) {
    let file_handle = rfd::AsyncFileDialog::new()
        .set_file_name(file_name)
        .add_filter("CSV", &["csv"])
        .save_file()
        .await;

    if let Some(handle) = file_handle {
        if let Err(e) = handle.write(content.as_bytes()).await {
            log::error!("Failed to write CSV: {e}");
        } else {
            log::info!("Exported CSV to {:?}", handle.file_name());
        }
    }
}
