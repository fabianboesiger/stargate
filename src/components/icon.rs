use dioxus::prelude::*;
use dioxus_free_icons::Icon as SvgIcon;
use dioxus_free_icons::icons::ld_icons;

#[derive(Debug, Clone, PartialEq)]
pub enum IconName {
    Table,
    List,
    RightFromBracket,
    Circle,
    Logo,
    Terminal,
    Bolt,
    ChartBar,
    Pencil,
    Trash,
    Plus,
    Lock,
    LockOpen,
    Download,
    Clock,
    Signal,
    Sun,
    Moon,
}

#[component]
pub fn Icon(name: IconName, #[props(default = "w-4 h-4".to_string())] class: String) -> Element {
    // Logo uses the Lucide "orbit" icon
    if name == IconName::Logo {
        return rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdOrbit,
            }
        };
    }

    match name {
        IconName::Table => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdTable,
            }
        },
        IconName::List => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdList,
            }
        },
        IconName::RightFromBracket => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdLogOut,
            }
        },
        IconName::Circle => rsx! {
            svg {
                class: "{class}",
                fill: "currentColor",
                view_box: "0 0 24 24",
                xmlns: "http://www.w3.org/2000/svg",
                circle { cx: "12", cy: "12", r: "12" }
            }
        },
        IconName::Terminal => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdTerminal,
            }
        },
        IconName::Bolt => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdZap,
            }
        },
        IconName::ChartBar => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdBarChart3,
            }
        },
        IconName::Pencil => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdPencil,
            }
        },
        IconName::Trash => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdTrash2,
            }
        },
        IconName::Plus => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdPlus,
            }
        },
        IconName::Lock => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdLock,
            }
        },
        IconName::LockOpen => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdLockOpen,
            }
        },
        IconName::Download => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdDownload,
            }
        },
        IconName::Clock => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdClock,
            }
        },
        IconName::Signal => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdSignalHigh,
            }
        },
        IconName::Sun => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdSun,
            }
        },
        IconName::Moon => rsx! {
            SvgIcon {
                class,
                width: None,
                height: None,
                icon: ld_icons::LdMoon,
            }
        },
        IconName::Logo => unreachable!(),
    }
}
