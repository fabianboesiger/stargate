use dioxus::prelude::*;

use super::TextInput;

/// Modal asking the user to activate premium (read-write) with a Polar license
/// key. Presentational only — activation/validation logic lives in
/// [`crate::polar`]; this component just collects the key and surfaces state.
#[component]
pub fn LicensePopup(
    /// Prefill the input with a previously stored key, if any.
    #[props(default)]
    initial_key: Option<String>,
    #[props(default = false)]
    busy: bool,
    #[props(default = None)]
    error: Option<String>,
    on_activate: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut key_input = use_signal(|| initial_key.unwrap_or_default());

    let submit = move || {
        let key = key_input.read().trim().to_string();
        if !key.is_empty() && !busy {
            on_activate.call(key);
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "bg-gray-900 border border-gray-700 rounded-xl p-6 w-[420px] max-w-[90vw] shadow-2xl",
                onclick: move |e| e.stop_propagation(),
                h3 { class: "text-sm font-medium text-gray-300 mb-1", "Activate Full Version" }
                p { class: "text-xs text-gray-500 mb-4",
                    "Read-write mode requires an active license. Enter your license key to unlock it on this device."
                }

                div {
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            submit();
                        }
                    },
                    TextInput {
                        value: key_input.read().clone(),
                        placeholder: "SG-XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX".to_string(),
                        oninput: move |e: FormEvent| key_input.set(e.value()),
                    }
                }

                if let Some(ref err) = error {
                    p { class: "text-xs text-red-600 mt-3", "{err}" }
                }
                p { class: "text-xs text-gray-600 mt-3",
                    "If your license is already in use on the maximum number of devices, deactivate one in your Polar account first."
                }

                div { class: "flex justify-between items-center gap-2 mt-4",
                    button {
                        class: "px-3 py-1.5 text-xs text-blue-400 hover:text-blue-300 transition-colors",
                        onclick: move |_| {
                            if let Err(e) = open::that(crate::polar::CHECKOUT_URL) {
                                log::error!("Failed to open checkout link: {e}");
                            }
                        },
                        "Get a license"
                    }
                    div { class: "flex gap-2",
                        button {
                            class: "px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md transition-colors",
                            onclick: move |_| on_cancel.call(()),
                            "Cancel"
                        }
                        button {
                            class: "px-3 py-1.5 text-xs text-gray-950 bg-blue-600 hover:bg-blue-500 rounded-md transition-colors disabled:opacity-50",
                            disabled: busy,
                            onclick: move |_| submit(),
                            if busy { "Activating..." } else { "Activate" }
                        }
                    }
                }
            }
        }
    }
}
