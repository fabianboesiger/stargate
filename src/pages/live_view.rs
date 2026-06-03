use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use crate::state::AppState;
use crate::ws::{self, TableUpdate};
use crate::components::{Icon, IconName, PageLayout, ErrorMessage};
use crate::Route;

#[derive(Debug, Clone)]
struct WsEvent {
    timestamp: String,
    kind: WsEventKind,
    table: String,
    row_count: usize,
    preview: String,
}

#[derive(Debug, Clone, PartialEq)]
enum WsEventKind {
    Initial,
    Insert,
    Delete,
    Error,
}

impl WsEventKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Initial => "INIT",
            Self::Insert => "INSERT",
            Self::Delete => "DELETE",
            Self::Error => "ERROR",
        }
    }

    fn color_class(&self) -> &'static str {
        match self {
            Self::Initial => "text-blue-400 bg-blue-500/10",
            Self::Insert => "text-emerald-600 bg-emerald-500/10",
            Self::Delete => "text-red-600 bg-red-500/10",
            Self::Error => "text-red-600 bg-red-500/10",
        }
    }
}

#[component]
pub fn LiveView(db_identity: String) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut events = use_signal(Vec::<WsEvent>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut stream_status = use_signal(|| "Connecting...".to_string());
    let mut auto_scroll = use_signal(|| true);
    let mut paused = use_signal(|| false);
    let mut bottom_anchor = use_signal(|| Option::<std::rc::Rc<MountedData>>::None);
    let mut events_per_sec = use_signal(|| 0.0f64);

    // Recording state: when enabled, all incoming events are streamed to a
    // JSON-lines file on disk. When stopped, `last_recording` holds the path so
    // the user can open the resulting file.
    let mut recording = use_signal(|| false);
    let mut recording_path = use_signal(|| Option::<PathBuf>::None);
    let mut last_recording = use_signal(|| Option::<PathBuf>::None);
    let mut recorded_count = use_signal(|| 0usize);

    // Snap the event list to the latest entry whenever new events arrive or
    // auto-scroll is (re)enabled.
    use_effect(move || {
        // Track these signals so the effect re-runs on change.
        let _ = events.read().len();
        let enabled = *auto_scroll.read();
        if enabled
            && let Some(anchor) = bottom_anchor.read().clone()
        {
            spawn(async move {
                let _ = anchor.scroll_to(ScrollBehavior::Instant).await;
            });
        }
    });

    let connected = app_state.read().connected;
    if !connected {
        navigator().push(Route::Login {});
        return rsx! {};
    }

    // Subscribe to all tables via WebSocket
    use_effect({
        let db_identity = db_identity.clone();
        move || {
            let db_identity = db_identity.clone();
            spawn(async move {
                let state = app_state.read();
                let Some(api) = state.api.clone() else {
                    loading.set(false);
                    stream_status.set("Disconnected".to_string());
                    return;
                };
                let base_url = state.server_url.clone();
                let token = if state.token.is_empty() { None } else { Some(state.token.clone()) };
                drop(state);

                // Fetch all table names from schema
                let schema = match api.get_schema(&db_identity).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to fetch schema: {e}");
                        error_msg.set(Some(format!("Failed to load schema: {e}")));
                        loading.set(false);
                        stream_status.set("Error".to_string());
                        return;
                    }
                };

                let table_names: Vec<String> = schema.tables.iter().map(|t| t.name.clone()).collect();
                if table_names.is_empty() {
                    loading.set(false);
                    stream_status.set("Live".to_string());
                    return;
                }

                log::info!("Live view subscribing to {} tables", table_names.len());

                // Subscribe to each table and merge into a shared channel
                let (merge_tx, mut merge_rx) = tokio::sync::mpsc::unbounded_channel::<TableUpdate>();

                for table_name in &table_names {
                    let mut rx = ws::subscribe_to_table(
                        &base_url,
                        &db_identity,
                        table_name,
                        token.as_deref(),
                    );
                    let merge_tx = merge_tx.clone();
                    tokio::spawn(async move {
                        while let Some(update) = rx.recv().await {
                            if merge_tx.send(update).is_err() {
                                break;
                            }
                        }
                    });
                }
                drop(merge_tx);

                stream_status.set("Connected".to_string());
                let mut received_initial = 0usize;
                let total_tables = table_names.len();

                // Coalesce the (potentially very high frequency) event stream: collect
                // events in a local buffer and append them to the reactive `events`
                // signal in a single batched write on a fixed cadence.
                let mut pending: Vec<WsEvent> = Vec::new();
                let mut flush = tokio::time::interval(tokio::time::Duration::from_millis(250));
                flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                // Active recording file writer (when recording is enabled).
                let mut recorder: Option<BufWriter<std::fs::File>> = None;

                loop {
                    tokio::select! {
                        biased;
                        maybe_update = merge_rx.recv() => {
                            let Some(update) = maybe_update else { break; };

                            sync_recorder(
                                &mut recorder,
                                *recording.read(),
                                recording_path,
                                last_recording,
                                recorded_count,
                            );

                            if *paused.read() {
                                continue;
                            }

                            let now = format_now();

                            let event = match update {
                                TableUpdate::InitialRows { table_name, rows } => {
                                    received_initial += 1;
                                    if received_initial >= total_tables {
                                        loading.set(false);
                                        stream_status.set("Live".to_string());
                                    }
                                    let preview = rows_preview(&rows);
                                    WsEvent {
                                        timestamp: now,
                                        kind: WsEventKind::Initial,
                                        table: table_name,
                                        row_count: rows.len(),
                                        preview,
                                    }
                                }
                                TableUpdate::Insert { table_name, rows } => {
                                    let preview = rows_preview(&rows);
                                    WsEvent {
                                        timestamp: now,
                                        kind: WsEventKind::Insert,
                                        table: table_name,
                                        row_count: rows.len(),
                                        preview,
                                    }
                                }
                                TableUpdate::Delete { table_name, rows } => {
                                    let preview = rows_preview(&rows);
                                    WsEvent {
                                        timestamp: now,
                                        kind: WsEventKind::Delete,
                                        table: table_name,
                                        row_count: rows.len(),
                                        preview,
                                    }
                                }
                                TableUpdate::Error(e) => {
                                    WsEvent {
                                        timestamp: now,
                                        kind: WsEventKind::Error,
                                        table: String::new(),
                                        row_count: 0,
                                        preview: e,
                                    }
                                }
                            };

                            if let Some(writer) = recorder.as_mut() {
                                write_event_line(writer, &event);
                                let prev = *recorded_count.read();
                                recorded_count.set(prev + 1);
                            }

                            pending.push(event);
                        }
                        _ = flush.tick() => {
                            sync_recorder(
                                &mut recorder,
                                *recording.read(),
                                recording_path,
                                last_recording,
                                recorded_count,
                            );

                            if !pending.is_empty() {
                                let count = pending.len() as f64;
                                // The flush interval is 250ms, so rate = count / 0.25
                                let rate = count / 0.25;
                                // Exponential moving average for smooth display
                                let prev = *events_per_sec.read();
                                let alpha = 0.3;
                                events_per_sec.set(prev * (1.0 - alpha) + rate * alpha);
                                push_events(&mut events, &mut pending);
                            } else {
                                // Decay towards zero when idle
                                let prev = *events_per_sec.read();
                                events_per_sec.set(prev * 0.7);
                            }
                        }
                    }
                }

                // Ensure any in-progress recording is finalized when the stream ends.
                if let Some(mut writer) = recorder.take() {
                    let _ = writer.flush();
                    if let Some(path) = recording_path.read().clone() {
                        last_recording.set(Some(path));
                    }
                    recording_path.set(None);
                    recording.set(false);
                }

                if stream_status.read().as_str() != "Disconnected" && stream_status.read().as_str() != "Error" {
                    log::warn!("Live view: all WebSocket connections ended");
                    stream_status.set("Disconnected".to_string());
                }
            });
        }
    });

    let header_status_color = match stream_status.read().as_str() {
        "Live" => "text-emerald-600 bg-emerald-500/10",
        "Error" => "text-red-600 bg-red-500/10",
        "Disconnected" => "text-gray-500 bg-gray-500/10",
        _ => "text-yellow-600 bg-yellow-500/10",
    };

    let all_events = events.read();
    let is_paused = *paused.read();
    let pause_class = if is_paused {
        "bg-emerald-600/20 text-emerald-700 hover:bg-emerald-600/30"
    } else {
        "bg-yellow-600/20 text-yellow-700 hover:bg-yellow-600/30"
    };

    let is_recording = *recording.read();
    let record_class = if is_recording {
        "bg-red-600/20 text-red-600 hover:bg-red-600/30"
    } else {
        "bg-gray-800 text-gray-400 hover:text-gray-200 hover:bg-gray-700"
    };
    let recorded = *recorded_count.read();
    let finished_recording = last_recording.read().clone();

    rsx! {
        PageLayout {
            db_identity: db_identity.clone(),
            active_page: "Live",
            title: "Live View".to_string(),
            header_extra: Some(rsx! {
                span { class: "inline-flex items-center gap-1.5 text-xs font-medium px-2.5 py-1 rounded-full {header_status_color}",
                    Icon { name: IconName::Circle, class: "w-1.5 h-1.5" }
                    "{stream_status}"
                }
            }),
            div { class: "px-8 pb-8 flex-1 min-h-0 flex flex-col gap-3",
                if let Some(err) = error_msg.read().as_ref() {
                    ErrorMessage { message: err.clone() }
                }

                // Toolbar
                div { class: "flex items-center gap-3",
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium transition-colors {pause_class}",
                        onclick: move |_| paused.set(!is_paused),
                        if is_paused {
                            "Resume"
                        } else {
                            "Pause"
                        }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg text-xs font-medium bg-gray-800 text-gray-400 hover:text-gray-200 hover:bg-gray-700 transition-colors",
                        onclick: move |_| events.set(Vec::new()),
                        "Clear"
                    }
                    button {
                        class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors {record_class}",
                        onclick: move |_| {
                            let next = !is_recording;
                            if next {
                                last_recording.set(None);
                            }
                            recording.set(next);
                        },
                        if is_recording {
                            Icon { name: IconName::Stop, class: "w-3 h-3" }
                            "Stop ({recorded})"
                        } else {
                            Icon { name: IconName::Circle, class: "w-2 h-2" }
                            "Record"
                        }
                    }
                    if !is_recording && let Some(path) = finished_recording {
                        {
                            let file_label = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "recording".to_string());
                            let open_path = path.clone();
                            rsx! {
                                button {
                                    class: "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-600/20 text-blue-500 hover:bg-blue-600/30 transition-colors",
                                    title: "{path.display()}",
                                    onclick: move |_| {
                                        reveal_in_file_manager(&open_path);
                                    },
                                    Icon { name: IconName::FileText, class: "w-3.5 h-3.5" }
                                    "Open {file_label}"
                                }
                            }
                        }
                    }
                    label { class: "flex items-center gap-1.5 text-xs text-gray-500 ml-auto cursor-pointer select-none",
                        input {
                            r#type: "checkbox",
                            checked: *auto_scroll.read(),
                            onchange: move |e: Event<FormData>| auto_scroll.set(e.checked()),
                            class: "rounded border-gray-700 bg-gray-800 text-blue-500 focus:ring-0 focus:ring-offset-0",
                        }
                        "Auto-scroll"
                    }
                    span { class: "text-xs text-gray-600", "{events_per_sec.read():.1} evt/s" }
                    span { class: "text-xs text-gray-600", "{all_events.len()} events" }
                }

                // Event list
                div { class: "flex-1 min-h-0 overflow-auto bg-gray-900 border border-gray-800 rounded-xl font-mono text-xs",
                    if *loading.read() {
                        div { class: "p-4 text-gray-500", "Connecting to tables..." }
                    }
                    for event in all_events.iter() {
                        div { class: "px-4 py-1.5 border-b border-gray-800/50 hover:bg-gray-800/30 flex items-start gap-3",
                            span { class: "text-gray-600 shrink-0 w-20", "{event.timestamp}" }
                            span { class: "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold shrink-0 w-14 justify-center {event.kind.color_class()}",
                                "{event.kind.label()}"
                            }
                            span { class: "text-gray-400 shrink-0 w-40 truncate", "{event.table}" }
                            if event.kind != WsEventKind::Error {
                                span { class: "text-gray-600 shrink-0 w-12 text-right",
                                    "{event.row_count}r"
                                }
                            }
                            span { class: "text-gray-500 truncate", "{event.preview}" }
                        }
                    }
                    // Bottom anchor used for auto-scrolling to the latest entry.
                    div { onmounted: move |e: Event<MountedData>| bottom_anchor.set(Some(e.data())) }
                }
            }
        }
    }
}

/// Append a batch of events in a single signal write, keeping a max of 1000
/// events to avoid unbounded memory growth. Drains `pending`.
fn push_events(events: &mut Signal<Vec<WsEvent>>, pending: &mut Vec<WsEvent>) {
    let mut current = events.read().clone();
    current.append(pending);
    if current.len() > 1000 {
        current.drain(..current.len() - 1000);
    }
    events.set(current);
}

/// Build a destination path for a new recording inside the application data
/// directory, creating the `recordings` folder if needed.
/// Open the file manager and select/highlight the given file.
/// - macOS:   `open -R <file>`
/// - Windows: `explorer /select,<file>`
/// - Linux:   falls back to opening the parent directory with `xdg-open`
fn reveal_in_file_manager(path: &PathBuf) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result = path
        .parent()
        .map(|dir| std::process::Command::new("xdg-open").arg(dir).spawn())
        .unwrap_or_else(|| {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no parent dir"))
        });

    match result {
        Ok(_) => log::info!("Revealed recording in file manager: {}", path.display()),
        Err(e) => log::error!("Failed to reveal recording {}: {e}", path.display()),
    }
}

fn new_recording_path() -> Option<PathBuf> {
    let dir = dirs::data_dir()?.join("stargate").join("recordings");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::error!("Failed to create recordings directory: {e}");
        return None;
    }
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    Some(dir.join(format!("live-{ts}.jsonl")))
}

/// Start or stop the recording file writer to match the requested `recording`
/// state, updating the associated signals.
fn sync_recorder(
    recorder: &mut Option<BufWriter<std::fs::File>>,
    recording: bool,
    mut recording_path: Signal<Option<PathBuf>>,
    mut last_recording: Signal<Option<PathBuf>>,
    mut recorded_count: Signal<usize>,
) {
    if recording && recorder.is_none() {
        let Some(path) = new_recording_path() else { return; };
        match std::fs::File::create(&path) {
            Ok(file) => {
                *recorder = Some(BufWriter::new(file));
                recorded_count.set(0);
                recording_path.set(Some(path.clone()));
                log::info!("Started recording live events to {}", path.display());
            }
            Err(e) => log::error!("Failed to create recording file {}: {e}", path.display()),
        }
    } else if !recording && recorder.is_some() {
        if let Some(mut writer) = recorder.take()
            && let Err(e) = writer.flush()
        {
            log::error!("Failed to flush recording file: {e}");
        }
        if let Some(path) = recording_path.read().clone() {
            last_recording.set(Some(path.clone()));
            log::info!("Stopped recording live events: {}", path.display());
        }
        recording_path.set(None);
    }
}

/// Serialize a single event as a JSON object and append it as one line.
fn write_event_line(writer: &mut BufWriter<std::fs::File>, event: &WsEvent) {
    let line = serde_json::json!({
        "timestamp": event.timestamp,
        "kind": event.kind.label(),
        "table": event.table,
        "row_count": event.row_count,
        "preview": event.preview,
    });
    match serde_json::to_string(&line) {
        Ok(s) => {
            if let Err(e) = writeln!(writer, "{s}") {
                log::error!("Failed to write recording line: {e}");
            }
        }
        Err(e) => log::error!("Failed to serialize recording line: {e}"),
    }
}

fn format_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    use chrono::{Local, TimeZone};
    match Local.timestamp_opt(secs as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%H:%M:%S").to_string(),
        _ => format!("{secs}"),
    }
}

fn rows_preview(rows: &[serde_json::Value]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let first = &rows[0];
    let s = serde_json::to_string(first).unwrap_or_default();
    if s.len() > 120 {
        format!("{}...", &s[..120])
    } else {
        s
    }
}
