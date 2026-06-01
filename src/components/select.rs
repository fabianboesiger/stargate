use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(
    options: Vec<SelectOption>,
    value: String,
    onchange: EventHandler<FormEvent>,
    placeholder: Option<String>,
) -> Element {
    rsx! {
        select {
            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-300 appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500/50",
            onchange: move |evt| onchange.call(evt),
            if let Some(placeholder) = &placeholder {
                option { value: "", selected: value.is_empty(), disabled: true, "{placeholder}" }
            }
            for opt in options.iter() {
                option { value: "{opt.value}", selected: value == opt.value, "{opt.label}" }
            }
        }
    }
}
