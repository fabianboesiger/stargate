use dioxus::prelude::*;
use crate::Route;
use super::{Icon, IconName};

#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    pub label: String,
    pub icon: IconName,
    pub route: Option<Route>,
    pub active: bool,
}

/// Build the standard nav items for a database page.
/// `active_page` should match one of: "Info", "Tables", "Reducers", "Logs", "SQL"
pub fn build_nav_items(db_identity: &str, active_page: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "Info".into(),
            icon: IconName::ChartBar,
            route: Some(Route::Info { db_identity: db_identity.to_string() }),
            active: active_page == "Info",
        },
        NavItem {
            label: "Tables".into(),
            icon: IconName::Table,
            route: Some(Route::DatabaseTables { db_identity: db_identity.to_string() }),
            active: active_page == "Tables",
        },
        NavItem {
            label: "Reducers".into(),
            icon: IconName::Bolt,
            route: Some(Route::Reducers { db_identity: db_identity.to_string() }),
            active: active_page == "Reducers",
        },
        NavItem {
            label: "Logs".into(),
            icon: IconName::List,
            route: Some(Route::DatabaseLogs { db_identity: db_identity.to_string() }),
            active: active_page == "Logs",
        },
        NavItem {
            label: "SQL".into(),
            icon: IconName::Terminal,
            route: Some(Route::Sql { db_identity: db_identity.to_string() }),
            active: active_page == "SQL",
        },
        NavItem {
            label: "Scheduled".into(),
            icon: IconName::Clock,
            route: Some(Route::ScheduledTasks { db_identity: db_identity.to_string() }),
            active: active_page == "Scheduled",
        },
        NavItem {
            label: "Live".into(),
            icon: IconName::Signal,
            route: Some(Route::LiveView { db_identity: db_identity.to_string() }),
            active: active_page == "Live",
        },
    ]
}

#[component]
pub fn Sidebar(
    nav_items: Vec<NavItem>,
    on_disconnect: EventHandler<MouseEvent>,
    #[props(default = false)]
    readonly: bool,
    #[props(default)]
    on_toggle_readonly: EventHandler<()>,
) -> Element {
    rsx! {
        aside { class: "w-56 bg-gray-900 border-r border-gray-800 flex flex-col",
            // App title
            div { class: "p-5",
                h1 { class: "text-lg font-bold text-white flex items-center gap-2.5",
                    Icon { name: IconName::Logo, class: "w-7 h-7 text-blue-400" }
                    "Stargate"
                }
            }

            // Navigation
            nav { class: "flex-1 px-3 space-y-0.5",
                for item in nav_items.iter() {
                    {
                        let classes = if item.active {
                            "flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium cursor-pointer transition-colors bg-blue-400/10 text-blue-300"
                        } else {
                            "flex items-center gap-3 px-3 py-2 rounded-md text-sm cursor-pointer transition-colors text-gray-500 hover:bg-white/[0.04]"
                        };
                        let icon_class = if item.active {
                            "w-4 h-4 shrink-0 text-blue-400"
                        } else {
                            "w-4 h-4 shrink-0"
                        };
                        let route = item.route.clone();
                        let icon = item.icon.clone();
                        rsx! {
                            div {
                                class: "{classes}",
                                onclick: move |_| {
                                    if let Some(ref r) = route {
                                        navigator().push(r.clone());
                                    }
                                },
                                Icon { name: icon.clone(), class: icon_class }
                                "{item.label}"
                            }
                        }
                    }
                }
            }

            // Readonly toggle + Disconnect button
            div { class: "px-3 py-4 border-t border-gray-800 space-y-1",
                button {
                    class: if readonly { "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium text-amber-400 bg-amber-500/10 hover:bg-amber-500/20 transition-colors" } else { "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-800/50 transition-colors" },
                    onclick: move |_| on_toggle_readonly.call(()),
                    Icon {
                        name: if readonly { IconName::Lock } else { IconName::LockOpen },
                        class: "w-3.5 h-3.5 shrink-0",
                    }
                    if readonly {
                        "Read-only"
                    } else {
                        "Read-write"
                    }
                }
                button {
                    class: "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-800/50 transition-colors",
                    onclick: move |evt| on_disconnect.call(evt),
                    Icon {
                        name: IconName::RightFromBracket,
                        class: "w-3.5 h-3.5 shrink-0",
                    }
                    "Disconnect"
                }
            }
        }
    }
}
