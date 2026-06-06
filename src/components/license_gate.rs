use std::time::Duration;

use dioxus::prelude::*;

use super::LicensePopup;
use crate::polar;
use crate::state::AppState;
use crate::storage::Storage;

/// How often a license is re-validated while read-write mode is enabled.
const CHECK_INTERVAL_SECS: u64 = 15 * 60;

/// Invisible app-root component that gates read-write mode behind a valid Polar
/// license.
///
/// While read-write is enabled it validates the stored license immediately and
/// then every [`CHECK_INTERVAL_SECS`]; the moment validation fails (or no
/// license is stored) it forces read-only mode and shows the activation popup.
/// No checks run while read-only is active. Mounted once at the app root so the
/// timer survives page navigation.
#[component]
pub fn LicenseGate() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let storage = use_context::<Storage>();

    let mut show_popup = use_signal(|| false);
    let mut popup_error = use_signal(|| Option::<String>::None);
    let mut popup_busy = use_signal(|| false);
    let mut check_task = use_signal(|| Option::<dioxus::core::Task>::None);
    let mut prev_readonly = use_signal(|| true);

    // React to read-only transitions: (re)start the periodic check when
    // read-write turns on, cancel it when read-only turns on.
    use_effect({
        let storage = storage.clone();
        move || {
            let readonly = app_state.read().readonly;
            // AppState changes for many reasons; only act on actual transitions.
            if readonly == *prev_readonly.peek() {
                return;
            }
            prev_readonly.set(readonly);

            if let Some(task) = check_task.write().take() {
                task.cancel();
            }

            if readonly {
                return; // Never check while read-only.
            }

            let storage = storage.clone();
            let task = spawn(async move {
                let Some(key) = storage.get_license_key() else {
                    // No license stored: block read-write and ask for one.
                    app_state.write().readonly = true;
                    popup_error.set(None);
                    show_popup.set(true);
                    return;
                };
                // `interval` fires its first tick immediately, then every period.
                let mut ticker = tokio::time::interval(Duration::from_secs(CHECK_INTERVAL_SECS));
                loop {
                    ticker.tick().await;
                    if polar::check_license(&key).await {
                        continue;
                    }
                    app_state.write().readonly = true;
                    popup_error.set(Some(
                        "Your license could not be verified on this device. Re-activate to continue in read-write mode.".to_string(),
                    ));
                    show_popup.set(true);
                    break;
                }
            });
            check_task.set(Some(task));
        }
    });

    if !*show_popup.read() {
        return rsx! {};
    }

    let stored_key = storage.get_license_key();

    rsx! {
        LicensePopup {
            initial_key: stored_key,
            busy: *popup_busy.read(),
            error: popup_error.read().clone(),
            on_activate: {
                let storage = storage.clone();
                move |key: String| {
                    let storage = storage.clone();
                    spawn(async move {
                        popup_busy.set(true);
                        popup_error.set(None);
                        storage.set_license_key(&key);
                        let ok = polar::check_license(&key).await;
                        popup_busy.set(false);
                        if ok {
                            show_popup.set(false);
                            // Enabling read-write (re)starts the periodic watcher.
                            app_state.write().readonly = false;
                        } else {
                            popup_error.set(Some(
                                "That license could not be activated. It may be invalid, expired, or already in use on the maximum number of devices.".to_string(),
                            ));
                        }
                    });
                }
            },
            on_cancel: move |_| {
                show_popup.set(false);
            },
        }
    }
}
