use dioxus::prelude::*;

/// Reusable page header with title and optional subtitle.
#[component]
pub fn PageHeader(
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] mono_subtitle: bool,
    /// Optional extra elements rendered inline after the title (e.g. status badges).
    #[props(default)]
    children: Element,
) -> Element {
    let subtitle_class = if mono_subtitle {
        "text-xs text-gray-600 font-mono mt-1"
    } else {
        "text-sm text-gray-500 mt-1"
    };

    rsx! {
        header { class: "px-8 pt-8 pb-6",
            div { class: "flex items-center gap-3",
                h2 { class: "text-2xl font-bold text-gray-50", "{title}" }
                {children}
            }
            if let Some(sub) = &subtitle {
                p { class: "{subtitle_class}", "{sub}" }
            }
        }
    }
}
