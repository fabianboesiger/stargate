use dioxus::prelude::*;

use crate::storage::Storage;
use crate::theme::Theme;

use super::{Icon, IconName};

/// A reusable control that switches between light and dark mode.
///
/// Reads and writes the shared [`Theme`] signal from context and persists the
/// choice in the local settings table, so the selection survives restarts.
#[component]
pub fn ThemeToggle(
    /// Extra classes appended to the button.
    #[props(default = String::new())]
    class: String,
) -> Element {
    let mut theme = use_context::<Signal<Theme>>();
    let storage = use_context::<Storage>();

    let is_dark = theme.read().is_dark();
    let (icon, label) = if is_dark {
        (IconName::Sun, "Light mode")
    } else {
        (IconName::Moon, "Dark mode")
    };

    rsx! {
        button {
            class: "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-800/50 transition-colors {class}",
            onclick: move |_| {
                let next = theme.read().toggled();
                theme.set(next);
                storage.set_theme(next);
                log::info!("Theme switched to {}", next.as_str());
            },
            Icon { name: icon, class: "w-3.5 h-3.5 shrink-0" }
            "{label}"
        }
    }
}
