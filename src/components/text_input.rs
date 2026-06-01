use dioxus::prelude::*;

#[component]
pub fn TextInput(
    value: String,
    oninput: EventHandler<FormEvent>,
    placeholder: Option<String>,
    input_type: Option<String>,
) -> Element {
    let placeholder = placeholder.unwrap_or_default();
    let input_type = input_type.unwrap_or_else(|| "text".into());

    rsx! {
        input {
            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-300 placeholder-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500/50",
            r#type: "{input_type}",
            placeholder: "{placeholder}",
            value: "{value}",
            oninput: move |evt| oninput.call(evt),
        }
    }
}
