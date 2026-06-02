use dioxus::prelude::*;

#[component]
pub fn Button(
    label: String,
    onclick: EventHandler<MouseEvent>,
    disabled: Option<bool>,
    class: Option<String>,
) -> Element {
    let disabled = disabled.unwrap_or(false);
    let extra_class = class.unwrap_or_default();

    rsx! {
        button {
            class: "w-full bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 disabled:cursor-not-allowed text-gray-950 font-medium py-2.5 px-4 rounded-lg transition-colors shadow-sm {extra_class}",
            disabled,
            onclick: move |evt| onclick.call(evt),
            "{label}"
        }
    }
}
