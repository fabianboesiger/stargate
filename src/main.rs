// The OpenAPI document in `openapi.rs` is built with deeply-nested `json!` literals.
#![recursion_limit = "512"]

mod api;
mod components;
mod config;
mod export;
mod openapi;
mod polar;
mod state;
mod storage;
mod theme;
mod pages;
mod ws;

use dioxus::prelude::*;
use components::LicenseGate;
use pages::{Login, DatabaseTables, DatabaseLogs, TableData, Sql, Reducers, Info, ScheduledTasks, LiveView, Schema};
use state::AppState;
use storage::Storage;

const STYLE: Asset = asset!("/assets/tailwind.css");

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[route("/")]
    Login {},
    #[route("/database/:db_identity/tables")]
    DatabaseTables { db_identity: String },
    #[route("/database/:db_identity/reducers")]
    Reducers { db_identity: String },
    #[route("/database/:db_identity/logs")]
    DatabaseLogs { db_identity: String },
    #[route("/database/:db_identity/sql")]
    Sql { db_identity: String },
    #[route("/database/:db_identity/info")]
    Info { db_identity: String },
    #[route("/database/:db_identity/schema")]
    Schema { db_identity: String },
    #[route("/database/:db_identity/scheduled")]
    ScheduledTasks { db_identity: String },
    #[route("/database/:db_identity/live")]
    LiveView { db_identity: String },
    #[route("/database/:db_identity/table/:table_name")]
    TableData { db_identity: String, table_name: String },
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .target(env_logger::Target::Stdout)
        .init();

    #[cfg(target_os = "macos")]
    set_macos_dock_icon();

    let icon = dioxus::desktop::icon_from_memory::<dioxus::desktop::tao::window::Icon>(
        include_bytes!("../assets/icon.png"),
    )
    .expect("Failed to load app icon");

    dioxus::LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_icon(icon)
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Stargate")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1280.0, 800.0)),
                ),
        )
        .launch(App);
}

#[cfg(target_os = "macos")]
fn set_macos_dock_icon() {
    use objc2::AllocAnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let icon_bytes = include_bytes!("../assets/icon.png");
    unsafe {
        let mtm = objc2::MainThreadMarker::new_unchecked();
        let data = NSData::with_bytes(icon_bytes);
        let image = NSImage::initWithData(NSImage::alloc(), &data);
        if let Some(image) = image {
            let app = NSApplication::sharedApplication(mtm);
            app.setApplicationIconImage(Some(&image));
        }
    }
}

#[component]
fn App() -> Element {
    use_context_provider(|| Signal::new(AppState::new()));
    let storage = use_context_provider(Storage::open);
    let theme = use_context_provider(|| Signal::new(storage.get_theme()));

    rsx! {
        document::Stylesheet { href: STYLE }
        div { class: "{theme.read().root_class()} contents",
            Router::<Route> {}
            LicenseGate {}
        }
    }
}
