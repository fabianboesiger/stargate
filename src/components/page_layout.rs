use dioxus::prelude::*;
use crate::state::AppState;
use crate::Route;
use super::{ConfirmPopup, ConfirmStyle, PageHeader, Sidebar, build_nav_items};

/// Shared page layout with sidebar, header, readonly toggle, and confirm popup.
#[component]
pub fn PageLayout(
    db_identity: String,
    active_page: &'static str,
    title: String,
    /// Optional elements rendered inline in the page header (e.g. live badges).
    #[props(default)]
    header_extra: Option<Element>,
    children: Element,
) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut confirm_readonly = use_signal(|| false);
    let mut trial_blocked = use_signal(|| false);

    let nav_items = build_nav_items(&db_identity, active_page);

    rsx! {
        div { class: "h-screen bg-gray-950 flex",
            Sidebar {
                nav_items,
                readonly: app_state.read().readonly,
                on_toggle_readonly: move |_| {
                    if cfg!(feature = "trial") {
                        // Read-write mode is not available in the trial version.
                        trial_blocked.set(true);
                    } else if app_state.read().readonly {
                        confirm_readonly.set(true);
                    } else {
                        app_state.write().readonly = true;
                    }
                },
                on_disconnect: move |_| {
                    *app_state.write() = AppState::new();
                    navigator().push(Route::Login {});
                },
            }

            main { class: "flex-1 overflow-hidden bg-gray-950 flex flex-col",
                PageHeader { title,
                    if let Some(extra) = header_extra {
                        {extra}
                    }
                }

                {children}
            }

            // Read-write confirm popup
            if *confirm_readonly.read() {
                ConfirmPopup {
                    title: "Disable Read-only Mode".to_string(),
                    message: "This will allow write operations to the database. Continue?".to_string(),
                    confirm_label: "Disable".to_string(),
                    style: ConfirmStyle::Warning,
                    on_cancel: move |_| {
                        confirm_readonly.set(false);
                    },
                    on_confirm: move |_| {
                        confirm_readonly.set(false);
                        app_state.write().readonly = false;
                    },
                }
            }

            // Trial mode block popup
            if *trial_blocked.read() {
                ConfirmPopup {
                    title: "Trial Mode".to_string(),
                    message: "Read-write mode is not supported in trial mode.".to_string(),
                    confirm_label: "OK".to_string(),
                    style: ConfirmStyle::Warning,
                    on_cancel: move |_| {
                        trial_blocked.set(false);
                    },
                    on_confirm: move |_| {
                        trial_blocked.set(false);
                    },
                }
            }
        }
    }
}
