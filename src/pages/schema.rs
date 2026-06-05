use dioxus::prelude::*;

use crate::api::{SchemaResponse, DEFAULT_AUTH_HOST};
use crate::components::{ErrorMessage, Icon, IconName, PageLayout};
use crate::openapi;
use crate::state::AppState;
use crate::theme::Theme;
use crate::Route;

/// Vendored RapiDoc bundle — a self-contained `<rapi-doc>` web component.
/// Loading it registers the custom element in the webview.
const RAPIDOC: Asset = asset!("/assets/rapidoc-min.js");

const RAPIDOC_ELEMENT_ID: &str = "stargate-rapidoc";

/// The "Schema" page: generates an OpenAPI 3.1 spec for the connected database
/// from its live schema, renders it with RapiDoc, and lets the user download it.
#[component]
pub fn Schema(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let theme = use_context::<Signal<Theme>>();

    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut spec = use_signal(|| Option::<serde_json::Value>::None);

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    // Fetch the schema and build the OpenAPI spec on mount.
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let db_identity = db_identity.clone();
            spawn(async move {
                let (api, base_url) = {
                    let state = app_state.read();
                    (state.api.clone(), state.server_url.clone())
                };
                let Some(api) = api else {
                    loading.set(false);
                    return;
                };

                // Build the spec from the live schema. If the schema can't be
                // fetched we still emit a spec for the static endpoints and show
                // a non-fatal note.
                let schema = match api.get_schema(&db_identity).await {
                    Ok(schema) => {
                        error_msg.set(None);
                        schema
                    }
                    Err(e) => {
                        log::error!("Failed to fetch schema for OpenAPI: {e}");
                        error_msg.set(Some(format!(
                            "Could not load reducers from the schema ({e}). Showing static endpoints only."
                        )));
                        SchemaResponse { tables: Vec::new(), reducers: Vec::new() }
                    }
                };

                let doc = openapi::build_spec(&base_url, DEFAULT_AUTH_HOST, &db_identity, &schema);
                spec.set(Some(doc));
                loading.set(false);
            });
        }
    });

    // Push the spec into the RapiDoc element (and re-apply theme colors) whenever
    // the spec or theme changes. A short poll covers the case where the custom
    // element hasn't finished upgrading yet.
    use_effect(move || {
        let Some(doc) = spec.read().clone() else { return };
        let dark = theme.read().is_dark();
        let spec_json = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".to_string());
        let (theme_name, bg, txt) = if dark {
            ("dark", "#141414", "#e5e5e5")
        } else {
            ("light", "#ffffff", "#2b2b2b")
        };

        let js = format!(
            r##"(function() {{
                var spec = {spec_json};
                function apply() {{
                    var el = document.getElementById("{id}");
                    if (!el || typeof el.loadSpec !== "function") {{ setTimeout(apply, 50); return; }}
                    el.setAttribute("theme", "{theme_name}");
                    el.setAttribute("bg-color", "{bg}");
                    el.setAttribute("text-color", "{txt}");
                    el.setAttribute("primary-color", "#3b82f6");
                    el.loadSpec(spec);
                }}
                apply();
            }})();"##,
            id = RAPIDOC_ELEMENT_ID,
        );

        spawn(async move {
            let _ = document::eval(&js).await;
        });
    });

    rsx! {
        // Register the RapiDoc custom element.
        document::Script { src: RAPIDOC }

        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Schema",
            title: "Schema".to_string(),
            header_extra: Some(rsx! {
                button {
                    class: "flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700/50 rounded-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed",
                    disabled: spec.read().is_none(),
                    onclick: move |_| {
                        if let Some(doc) = spec.read().clone() {
                            let json = serde_json::to_string_pretty(&doc).unwrap_or_default();
                            spawn(async move {
                                download_spec("openapi.json", &json).await;
                            });
                        }
                    },
                    Icon { name: IconName::Download, class: "w-3.5 h-3.5" }
                    "Download"
                }
            }),
            div { class: "flex-1 min-h-0 flex flex-col",
                if *loading.read() {
                    div { class: "px-8 pb-8 text-gray-500 text-sm", "Generating schema..." }
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div { class: "px-8 pb-4",
                        ErrorMessage { message: err.clone() }
                    }
                }

                // RapiDoc renders into this element via `loadSpec` (see effect above).
                // It uses Shadow DOM, so its styles are isolated from the app.
                div {
                    class: "flex-1 min-h-0",
                    dangerous_inner_html: r#"<rapi-doc id="stargate-rapidoc" render-style="read" show-header="false" allow-spec-url-load="false" allow-spec-file-load="false" allow-server-selection="true" allow-authentication="true" allow-try="true" style="width:100%;height:100%;"></rapi-doc>"#,
                }
            }
        }
    }
}

/// Save the spec to disk via the native file dialog (mirrors the CSV export flow).
async fn download_spec(file_name: &str, content: &str) {
    let file_handle = rfd::AsyncFileDialog::new()
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
        .save_file()
        .await;

    if let Some(handle) = file_handle {
        if let Err(e) = handle.write(content.as_bytes()).await {
            log::error!("Failed to write OpenAPI spec: {e}");
        } else {
            log::info!("Exported OpenAPI spec to {:?}", handle.file_name());
        }
    }
}
