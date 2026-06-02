use dioxus::prelude::*;

#[component]
pub fn ErrorMessage(message: String) -> Element {
    rsx! {
        div { class: "flex items-start gap-3 bg-red-500/10 border border-red-500/20 rounded-lg px-4 py-3",
            span { class: "text-red-600 text-sm mt-0.5", "!" }
            p { class: "text-sm text-red-600", "{message}" }
        }
    }
}
