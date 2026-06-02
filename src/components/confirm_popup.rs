use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmStyle {
    Danger,
    Warning,
}

#[component]
pub fn ConfirmPopup(
    title: String,
    message: String,
    confirm_label: String,
    #[props(default = ConfirmStyle::Warning)]
    style: ConfirmStyle,
    #[props(default = false)]
    loading: bool,
    #[props(default = None)]
    error: Option<String>,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let btn_class = match style {
        ConfirmStyle::Danger => "px-3 py-1.5 text-xs text-white bg-red-600 hover:bg-red-500 rounded-md transition-colors disabled:opacity-50",
        ConfirmStyle::Warning => "px-3 py-1.5 text-xs text-gray-950 bg-blue-600 hover:bg-blue-500 rounded-md transition-colors disabled:opacity-50",
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "bg-gray-900 border border-gray-700 rounded-xl p-6 w-[420px] max-w-[90vw] shadow-2xl",
                onclick: move |e| e.stop_propagation(),
                h3 { class: "text-sm font-medium text-gray-300 mb-1", "{title}" }
                p { class: "text-xs text-gray-500 mb-4", "{message}" }

                if let Some(ref err) = error {
                    p { class: "text-xs text-red-600 mb-3", "{err}" }
                }

                div { class: "flex justify-end gap-2",
                    button {
                        class: "px-3 py-1.5 text-xs text-gray-400 hover:text-gray-200 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md transition-colors",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "{btn_class}",
                        disabled: loading,
                        onclick: move |_| on_confirm.call(()),
                        if loading {
                            "Processing..."
                        } else {
                            "{confirm_label}"
                        }
                    }
                }
            }
        }
    }
}
