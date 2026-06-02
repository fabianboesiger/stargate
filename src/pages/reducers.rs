use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::{ReducerParam, ReducerSchema};
use crate::components::{ConfirmPopup, ConfirmStyle, DataTable, DATA_TABLE_PAGE_SIZE, ErrorMessage, IconName, PageLayout, TableActionButton};
use crate::Route;

#[derive(Debug, Clone, PartialEq)]
enum ConfirmAction {
    CallReducer,
}

#[component]
pub fn Reducers(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut reducers = use_signal(Vec::<ReducerSchema>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut search = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let mut selected_reducer = use_signal(|| Option::<String>::None);
    let mut param_values = use_signal(Vec::<String>::new);
    let mut call_result = use_signal(|| Option::<String>::None);
    let mut call_error = use_signal(|| Option::<String>::None);
    let mut calling = use_signal(|| false);
    let mut trigger_call = use_signal(|| 0u32);
    let mut confirm_action = use_signal(|| Option::<ConfirmAction>::None);

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    // Fetch schema on mount
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let db_identity = db_identity.clone();
            spawn(async move {
                let state = app_state.read();
                if let Some(api) = &state.api {
                    let api = api.clone();
                    drop(state);

                    match api.get_schema(&db_identity).await {
                        Ok(schema) => {
                            reducers.set(schema.reducers);
                            error_msg.set(None);
                        }
                        Err(e) => {
                            log::error!("Failed to fetch schema: {e}");
                            error_msg.set(Some(format!("Failed to load reducers: {e}")));
                        }
                    }
                }
                loading.set(false);
            });
        }
    });

    let current_page = *page.read();

    // Call reducer when triggered
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let _trigger = *trigger_call.read();
            if _trigger == 0 {
                return;
            }

            let reducer_name = match selected_reducer.peek().as_ref() {
                Some(name) => name.clone(),
                None => return,
            };
            let values = param_values.peek().clone();

            // Get the param types from schema
            let all_reducers = reducers.peek();
            let reducer_schema = all_reducers.iter().find(|r| r.name == reducer_name);
            let params = reducer_schema.map(|r| r.params()).unwrap_or_default();

            // Build args array from individual field values
            let mut args: Vec<serde_json::Value> = Vec::new();
            for (i, val_str) in values.iter().enumerate() {
                let param_type = params.get(i).map(|p| type_hint(&p.ty)).unwrap_or("string");
                let parsed = parse_param_value(val_str, param_type);
                match parsed {
                    Ok(v) => args.push(v),
                    Err(e) => {
                        let name = params.get(i).map(|p| p.name.as_str()).unwrap_or("?");
                        call_error.set(Some(format!("Invalid value for '{name}': {e}")));
                        return;
                    }
                }
            }

            calling.set(true);
            call_error.set(None);
            call_result.set(None);

            let db_id = db_identity.clone();
            spawn(async move {
                let state = app_state.read();
                if let Some(api) = &state.api {
                    let api = api.clone();
                    drop(state);

                    match api.call_reducer(&db_id, &reducer_name, &args).await {
                        Ok(response) => {
                            log::info!("Reducer {reducer_name} called successfully");
                            if response.is_empty() {
                                call_result.set(Some("Reducer executed successfully.".into()));
                            } else {
                                call_result.set(Some(response));
                            }
                            call_error.set(None);
                        }
                        Err(e) => {
                            log::error!("Reducer call failed: {e}");
                            call_error.set(Some(e));
                        }
                    }
                }
                calling.set(false);
            });
        }
    });

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Reducers",
            title: "Reducers".to_string(),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col gap-4",
                if *loading.read() {
                    div { class: "text-gray-500 text-sm", "Loading..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    ErrorMessage { message: err.clone() }
                }

                // Executor panel (shown when a reducer is selected)
                if let Some(ref name) = *selected_reducer.read() {
                    div { class: "bg-gray-900 border border-gray-800 rounded-xl p-5 flex flex-col gap-3 shrink-0",
                        div { class: "flex items-center justify-between",
                            h3 { class: "text-sm font-medium text-gray-50 font-mono",
                                "{name}"
                            }
                            button {
                                class: "text-xs text-gray-500 hover:text-gray-300 transition-colors",
                                onclick: move |_| {
                                    selected_reducer.set(None);
                                    call_result.set(None);
                                    call_error.set(None);
                                },
                                "Close"
                            }
                        }

                        // Parameter fields
                        {
                            let all_reducers = reducers.read();
                            let reducer = all_reducers.iter().find(|r| &r.name == name);
                            let params = reducer.map(|r| r.params()).unwrap_or_default();
                            if params.is_empty() {
                                rsx! {
                                    div { class: "text-xs text-gray-500 italic", "No parameters" }
                                }
                            } else {
                                let current_values = param_values.read();
                                rsx! {
                                    div { class: "flex flex-col gap-2",
                                        for (i , param) in params.iter().enumerate() {
                                            ReducerParamField {
                                                key: "{name}-{i}",
                                                index: i,
                                                param: param.clone(),
                                                value: current_values.get(i).cloned().unwrap_or_default(),
                                                on_change: move |val: String| {
                                                    let mut vals = param_values.peek().clone();
                                                    if i < vals.len() {
                                                        vals[i] = val;
                                                    }
                                                    param_values.set(vals);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Execute button
                        div { class: "flex items-center gap-3 pt-1",
                            button {
                                class: "px-4 py-2 bg-blue-600 hover:bg-blue-500 text-gray-950 text-xs font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap",
                                disabled: *calling.read() || app_state.read().readonly,
                                onclick: move |_| {
                                    confirm_action.set(Some(ConfirmAction::CallReducer));
                                },
                                if *calling.read() {
                                    "Calling..."
                                } else {
                                    "Execute"
                                }
                            }
                        }

                        if let Some(ref err) = *call_error.read() {
                            div { class: "text-xs text-red-600 bg-red-500/10 rounded-md px-3 py-2",
                                "{err}"
                            }
                        }
                        if let Some(ref res) = *call_result.read() {
                            div { class: "text-xs text-emerald-600 bg-emerald-500/10 rounded-md px-3 py-2 font-mono",
                                "{res}"
                            }
                        }
                    }
                }

                // Reducers table
                if !*loading.read() && !reducers.read().is_empty() {
                    {
                        let query = search.read().to_lowercase();
                        let all_reducers = reducers.read();
                        let filtered: Vec<&ReducerSchema> = if query.is_empty() {
                            all_reducers.iter().collect()
                        } else {
                            all_reducers
                                .iter()
                                .filter(|r| { r.name.to_lowercase().contains(&query) })
                                .collect()
                        };
                        let total = filtered.len();
                        let start = current_page * DATA_TABLE_PAGE_SIZE;
                        let page_reducers: Vec<&ReducerSchema> = filtered
                            .into_iter()
                            .skip(start)
                            .take(DATA_TABLE_PAGE_SIZE)
                            .collect();
                        rsx! {
                            DataTable {
                                columns: vec![
                                    ("Name".into(), String::new()),
                                    ("Parameters".into(), String::new()),
                                    ("".into(), String::new()),
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
                                            placeholder: "Filter reducers...",
                                            value: "{search}",
                                            oninput: move |e| {
                                                search.set(e.value());
                                                page.set(0);
                                            },
                                        }
                                        span { class: "text-xs text-gray-600", "{total} reducers" }
                                    }
                                },
                                for reducer in page_reducers.iter() {
                                    ReducerRow {
                                        reducer: (*reducer).clone(),
                                        on_select: move |name: String| {
                                            // Initialize param_values to the correct number of fields
                                            let all = reducers.peek();
                                            let param_count = all
                                                .iter()
                                                .find(|r| r.name == name)
                                                .map(|r| r.params().len())
                                                .unwrap_or(0);
                                            param_values.set(vec![String::new(); param_count]);
                                            selected_reducer.set(Some(name));
                                            call_result.set(None);
                                            call_error.set(None);
                                        },
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
                    title: "Confirm Reducer Call".to_string(),
                    message: "This will execute a reducer which may modify the database. Continue?".to_string(),
                    confirm_label: "Execute".to_string(),
                    style: ConfirmStyle::Warning,
                    on_cancel: move |_| {
                        confirm_action.set(None);
                    },
                    on_confirm: move |_| {
                        confirm_action.set(None);
                        trigger_call.set(trigger_call() + 1);
                    },
                }
            }
        }
    }
}

#[component]
fn ReducerRow(reducer: ReducerSchema, on_select: EventHandler<String>) -> Element {
    let params = reducer.params();
    let param_count = params.len();
    let params_display = if params.is_empty() {
        "none".to_string()
    } else {
        params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ")
    };
    let name = reducer.name.clone();

    rsx! {
        tr { class: "border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors",
            td { class: "px-5 py-3 text-sm text-gray-200 font-mono", "{reducer.name}" }
            td { class: "px-5 py-3 text-xs text-gray-500",
                if param_count == 0 {
                    span { class: "text-gray-600 italic", "none" }
                } else {
                    span { class: "font-mono", "{params_display}" }
                }
            }
            td { class: "px-5 py-3 text-right",
                TableActionButton {
                    label: "Call".to_string(),
                    icon: IconName::Bolt,
                    onclick: move |_| on_select.call(name.clone()),
                }
            }
        }
    }
}

/// Determine a simple type hint from the schema type value
fn type_hint(ty: &serde_json::Value) -> &'static str {
    // Handle SpacetimeDB algebraic type format
    if let Some(obj) = ty.as_object() {
        if obj.contains_key("U8") || obj.contains_key("U16") || obj.contains_key("U32")
            || obj.contains_key("U64") || obj.contains_key("U128")
            || obj.contains_key("I8") || obj.contains_key("I16") || obj.contains_key("I32")
            || obj.contains_key("I64") || obj.contains_key("I128")
            || obj.contains_key("F32") || obj.contains_key("F64")
        {
            return "number";
        }
        if obj.contains_key("Bool") {
            return "bool";
        }
        if obj.contains_key("String") {
            return "string";
        }
    }
    if let Some(s) = ty.as_str() {
        return match s {
            "U8" | "U16" | "U32" | "U64" | "U128" | "I8" | "I16" | "I32" | "I64" | "I128"
            | "F32" | "F64" => "number",
            "Bool" => "bool",
            "String" => "string",
            _ => "json",
        };
    }
    "json"
}

/// Parse a string value into a JSON value based on its type hint
fn parse_param_value(val: &str, hint: &str) -> Result<serde_json::Value, String> {
    if val.is_empty() {
        return Err("value is required".into());
    }
    match hint {
        "number" => {
            if let Ok(n) = val.parse::<i64>() {
                Ok(serde_json::Value::Number(n.into()))
            } else if let Ok(f) = val.parse::<f64>() {
                Ok(serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null))
            } else {
                Err("expected a number".into())
            }
        }
        "bool" => match val.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(serde_json::Value::Bool(true)),
            "false" | "0" | "no" => Ok(serde_json::Value::Bool(false)),
            _ => Err("expected true or false".into()),
        },
        "string" => Ok(serde_json::Value::String(val.to_string())),
        _ => {
            // Try to parse as JSON, fall back to string
            serde_json::from_str(val)
                .or_else(|_| Ok(serde_json::Value::String(val.to_string())))
        }
    }
}

/// Get a display label for the type
fn type_label(ty: &serde_json::Value) -> String {
    if let Some(obj) = ty.as_object() {
        for key in ["U8", "U16", "U32", "U64", "U128", "I8", "I16", "I32", "I64", "I128",
                    "F32", "F64", "Bool", "String"] {
            if obj.contains_key(key) {
                return key.to_lowercase();
            }
        }
    }
    if let Some(s) = ty.as_str() {
        return s.to_lowercase();
    }
    "json".to_string()
}

#[component]
fn ReducerParamField(
    index: usize,
    param: ReducerParam,
    value: String,
    on_change: EventHandler<String>,
) -> Element {
    let hint = type_hint(&param.ty);
    let label = type_label(&param.ty);
    let name_display = if param.name.is_empty() {
        format!("arg{index}")
    } else {
        param.name.clone()
    };

    let input_type = match hint {
        "number" => "number",
        _ => "text",
    };

    rsx! {
        div { class: "flex items-center gap-3",
            label {
                class: "text-xs text-gray-400 w-28 shrink-0 font-mono truncate",
                title: "{name_display}",
                "{name_display}"
            }
            if hint == "bool" {
                select {
                    class: "flex-1 bg-gray-800 border border-gray-700/50 rounded-md px-3 py-1.5 text-xs text-gray-300 focus:outline-none focus:border-blue-500/50",
                    value: "{value}",
                    onchange: move |e| on_change.call(e.value()),
                    option {
                        value: "",
                        disabled: true,
                        selected: value.is_empty(),
                        "Select..."
                    }
                    option { value: "true", "true" }
                    option { value: "false", "false" }
                }
            } else {
                input {
                    class: "flex-1 bg-gray-800 border border-gray-700/50 rounded-md px-3 py-1.5 text-xs text-gray-300 font-mono placeholder-gray-600 focus:outline-none focus:border-blue-500/50",
                    r#type: input_type,
                    placeholder: "{label}",
                    value: "{value}",
                    oninput: move |e| on_change.call(e.value()),
                }
            }
            span { class: "text-[10px] text-gray-600 w-14 shrink-0 text-right", "{label}" }
        }
    }
}
