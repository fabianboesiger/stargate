use dioxus::prelude::*;

use super::{Icon, IconName};

/// Visual intent of a [`TableActionButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableActionVariant {
    /// A neutral action that accents in the app's primary color on hover.
    #[default]
    Default,
    /// A destructive action that accents red on hover.
    Danger,
}

/// A small, consistent action button for use inside table rows.
///
/// Centralizes the look of every in-table action (icon + label pill) so they
/// stay visually identical across pages.
#[component]
pub fn TableActionButton(
    label: String,
    icon: IconName,
    onclick: EventHandler<MouseEvent>,
    #[props(default = false)] disabled: bool,
    #[props(default)] variant: TableActionVariant,
    #[props(default)] title: Option<String>,
) -> Element {
    const BASE: &str =
        "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium transition-colors";

    let class = if disabled {
        format!("{BASE} bg-gray-800/60 text-gray-600 cursor-not-allowed")
    } else {
        match variant {
            TableActionVariant::Default => {
                format!("{BASE} bg-gray-800 text-gray-400 hover:bg-blue-600 hover:text-gray-950")
            }
            TableActionVariant::Danger => {
                format!("{BASE} bg-gray-800 text-gray-400 hover:bg-red-600/80 hover:text-red-100")
            }
        }
    };

    rsx! {
        button {
            class,
            disabled,
            title,
            onclick: move |evt| onclick.call(evt),
            Icon { name: icon, class: "w-3 h-3" }
            "{label}"
        }
    }
}
