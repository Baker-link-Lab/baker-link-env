#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cmd;
mod logger;
mod parameter;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::trayicon::{self, menu as tray_menu};
use dioxus::prelude::*;
use log::LevelFilter;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const INLINE_CSS: &str = include_str!("../assets/main.css");
const LOGO_PNG_BYTES: &[u8] = include_bytes!("../assets/baker-link-logo.png");

use base64::Engine as _;
fn logo_data_uri() -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(LOGO_PNG_BYTES);
    format!("data:image/png;base64,{b64}")
}
const HISTORY_MAX: usize = 10;

static DISPLAY_BUFFER: OnceLock<Mutex<logger::DisplayBuffer>> = OnceLock::new();
static DAP_SERVER: OnceLock<Mutex<cmd::ProbeRsDapServer>> = OnceLock::new();

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct HistoryEntry {
    name: String,
    path: String, // full path including project name
}

fn display_buffer() -> &'static Mutex<logger::DisplayBuffer> {
    DISPLAY_BUFFER.get_or_init(|| Mutex::new(logger::DisplayBuffer::new()))
}

fn dap_server() -> &'static Mutex<cmd::ProbeRsDapServer> {
    DAP_SERVER.get_or_init(|| Mutex::new(cmd::ProbeRsDapServer::default()))
}

fn main() {
    let mut logger = env_logger::Builder::new();
    if std::env::var("RUST_LOG").is_ok() {
        logger.parse_default_env();
    } else {
        logger.filter_level(LevelFilter::Off);
    }
    let _ = logger.try_init();
    let icon = load_window_icon();
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_icon(icon)
                .with_window(
                    WindowBuilder::new()
                        .with_title(parameter::APP_NAME)
                        .with_inner_size(LogicalSize::new(920.0, 680.0)),
                )
                // --- System tray / menu bar support ---
                // ×ボタンでウィンドウを閉じずに非表示にする
                .with_close_behaviour(
                    dioxus::desktop::WindowCloseBehaviour::WindowHides,
                )
                // 全ウィンドウが非表示でもプロセスを終了しない（トレイ常駐）
                .with_exits_when_last_window_closes(false)
                // トレイアイコン左クリックでウィンドウを再表示する（デフォルトtrue、明示）
                .with_tray_icon_show_window_on_click(true),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    // ===== System tray icon setup (runs once on mount) =====
    // トレイメニューを構築: Open / Hide / Quit
    let menu_open = tray_menu::MenuItem::new("Open", true, None);
    let menu_hide = tray_menu::MenuItem::new("Hide", true, None);
    let menu_quit = tray_menu::MenuItem::new("Quit", true, None);

    // メニューID を保持（イベントハンドラで照合に使う）
    let id_open = menu_open.id().clone();
    let id_hide = menu_hide.id().clone();
    let id_quit = menu_quit.id().clone();

    let tray_menu_obj = tray_menu::Menu::new();
    let _ = tray_menu_obj.append_items(&[
        &menu_open,
        &tray_menu::PredefinedMenuItem::separator(),
        &menu_hide,
        &tray_menu::PredefinedMenuItem::separator(),
        &menu_quit,
    ]);

    // アイコンを icon/icon.png から読み込み（tray_icon::Icon 形式）
    let tray_icon = {
        let bytes = include_bytes!("../icon/icon.png");
        let img = image::load_from_memory(bytes)
            .expect("Failed to load tray icon")
            .into_rgba8();
        let (w, h) = img.dimensions();
        trayicon::Icon::from_rgba(img.into_raw(), w, h)
            .expect("Failed to create tray icon")
    };

    // トレイアイコンを初期化（Dioxus コンテキストに登録される）
    trayicon::init_tray_icon(tray_menu_obj, Some(tray_icon));

    // トレイメニュー項目のクリックハンドラ
    dioxus::desktop::use_tray_menu_event_handler(move |event: &tray_menu::MenuEvent| {
        let desktop = dioxus::desktop::window();
        if event.id() == &id_open {
            // Open: ウィンドウを表示して前面に出す
            desktop.set_visible(true);
            desktop.set_focus();
        } else if event.id() == &id_hide {
            // Hide: ウィンドウを非表示にする
            desktop.set_visible(false);
        } else if event.id() == &id_quit {
            // Quit: アプリを完全終了する
            // DAP サーバーが起動中なら停止
            if let Ok(mut server) = dap_server().lock() {
                server.stop();
            }
            // トレイアイコンを明示的に削除（drop しないとシェルに残る）
            if let Some(tray) = trayicon::use_tray_icon() {
                drop(tray);
            }
            std::process::exit(0);
        }
    });
    let mut project_name = use_signal(|| "myproject".to_string());
    let mut vscode_open_enabled = use_signal(|| true);
    let mut dap_port = use_signal(|| "50001".to_string());
    let mut dap_running = use_signal(|| false);
    let mut logs = use_signal(Vec::<String>::new);
    let mut docker_status = use_signal(|| "Docker: ?".to_string());
    let mut last_error = use_signal(|| Option::<String>::None);
    let mut log_tick = use_signal(|| 0_u64);
    let mut docker_prompt_dismissed = use_signal(|| false);
    let mut history = use_signal(|| load_history());
    let mut show_history = use_signal(|| false);
    let mut show_splash = use_signal(|| should_show_splash());

    // Auto-dismiss splash after 3 seconds
    use_future(move || async move {
        if *show_splash.peek() {
            mark_splash_shown();
            tokio::time::sleep(Duration::from_millis(3000)).await;
            show_splash.set(false);
        }
    });

    // Poll logs every 300ms
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            log_tick += 1;
        }
    });

    {
        use_effect(move || {
            let _ = log_tick();
            if let Ok(mut buffer) = display_buffer().lock() {
                buffer.channel_recv();
                let latest = buffer.buffer.clone();
                if latest != *logs.read() {
                    logs.set(latest);
                }
            }
        });
    }

    // Docker status auto-polling every 5 seconds
    use_future(move || async move {
        loop {
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(cmd::is_docker_running());
            });
            if let Ok(result) = rx.await {
                let new_status = match result {
                    Ok(true) => "Docker: On".to_string(),
                    Ok(false) => "Docker: Off".to_string(),
                    Err(_) => "Docker: ?".to_string(),
                };
                if *docker_status.read() != new_status {
                    // Reset dismissed flag when Docker comes back online
                    if new_status.contains("On") {
                        docker_prompt_dismissed.set(false);
                    }
                    docker_status.set(new_status);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    rsx! {
        document::Title { "{parameter::APP_NAME}" }
        document::Style { {INLINE_CSS} }

        div {
            class: "app-shell",

            // ===== TOP BAR =====
            div {
                class: "top-bar",

                // Left: brand + Docker status
                div {
                    class: "top-bar-left",
                    div { class: "brand-icon", "B" }
                    span { class: "brand-name", "{parameter::APP_NAME}" }
                    span { class: "brand-version", "{parameter::build_version_label()}" }

                    // Docker status (separated by border)
                    div {
                        class: "docker-status",
                        span { class: "{docker_dot_class(&docker_status.read())}" }
                        span { class: "docker-label", "{docker_status}" }
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let status_text = match cmd::is_docker_running() {
                                    Ok(true) => "Docker: On".to_string(),
                                    Ok(false) => "Docker: Off".to_string(),
                                    Err(e) => {
                                        if let Ok(mut buffer) = display_buffer().lock() {
                                            buffer.log_error(e.clone());
                                        }
                                        last_error.set(Some(e));
                                        "Docker: ?".to_string()
                                    }
                                };
                                docker_status.set(status_text);
                            },
                            "Refresh"
                        }
                    }
                }

                // Spacer
                div { class: "top-bar-spacer" }

                // Right: History + Help
                div {
                    class: "top-bar-right",

                    // History dropdown
                    div {
                        class: "dropdown-container",
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let current = *show_history.read();
                                show_history.set(!current);
                            },
                            "History"
                        }
                        if *show_history.read() {
                            div {
                                class: "dropdown-menu",
                                if history.read().is_empty() {
                                    div { class: "dropdown-empty", "No history yet" }
                                }
                                for (idx, entry) in history.read().iter().enumerate() {
                                    {
                                        let entry_path = entry.path.clone();
                                        let entry_name = entry.name.clone();
                                        let entry_idx = idx;
                                        rsx! {
                                            button {
                                                class: "dropdown-item",
                                                onclick: move |_| {
                                                    show_history.set(false);
                                                    if std::path::Path::new(&entry_path).exists() {
                                                        let _ = cmd::start_rd();
                                                        if let Ok(mut buffer) = display_buffer().lock() {
                                                            buffer.log_info(format!("Visual Studio Code opened: {}", entry_path));
                                                        }
                                                        if let Err(e) = cmd::open_vscode(&entry_path) {
                                                            if let Ok(mut buffer) = display_buffer().lock() {
                                                                buffer.log_error(format!("Visual Studio Code failed to open: {}", e));
                                                            }
                                                        }
                                                    } else {
                                                        // Remove missing entry
                                                        if let Ok(mut buffer) = display_buffer().lock() {
                                                            buffer.log_error(format!("Project not found: {}", entry_path));
                                                        }
                                                        last_error.set(Some(format!("Project not found: {}", entry_path)));
                                                        let mut h = history.read().clone();
                                                        if entry_idx < h.len() {
                                                            h.remove(entry_idx);
                                                            save_history(&h);
                                                            history.set(h);
                                                        }
                                                    }
                                                },
                                                div { class: "dropdown-item-name", "{entry_name}" }
                                                div { class: "dropdown-item-path", "{entry_path}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "btn-chip",
                        onclick: move |_| {
                            let _ = open::that("https://github.com/Baker-link-Lab/baker-link-env/blob/main/README.md");
                        },
                        "Help"
                    }
                }
            }

            // ===== MAIN CONTENT =====
            main {
                class: "main-content",

                // Two-column grid: Create Project + DAP Server
                div {
                    class: "cards-grid",

                    // ---- Create Project ----
                    section {
                        class: "card",
                        h2 { class: "section-title", "Create Project" }
                        p { class: "section-subtitle", "Generate a template-based project and open it in VS Code." }

                        div {
                            class: "card-body",
                            div {
                                class: "input-row",
                                label { class: "input-label", "Project name" }
                                input {
                                    class: "input",
                                    value: "{project_name}",
                                    oninput: move |ev| project_name.set(ev.value()),
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            let path_str = path.to_string_lossy().to_string();
                                            let joined = std::path::Path::new(&path_str).join(project_name.read().as_str());
                                            let joined_str = joined.to_string_lossy().to_string();

                                            if !joined.exists() {
                                                match cmd::generate_project(project_name.read().as_str(), &path_str) {
                                                    Ok(_) => {
                                                        if let Ok(mut buffer) = display_buffer().lock() {
                                                            buffer.log_info(format!("Project {} generated", joined_str));
                                                            buffer.channel_recv();
                                                            logs.set(buffer.buffer.clone());
                                                        }
                                                        // Push to history
                                                        let mut h = history.read().clone();
                                                        if !h.iter().any(|e| e.path == joined_str) {
                                                            if h.len() >= HISTORY_MAX {
                                                                h.remove(0);
                                                            }
                                                            h.push(HistoryEntry {
                                                                name: project_name.read().clone(),
                                                                path: joined_str.clone(),
                                                            });
                                                            save_history(&h);
                                                            history.set(h);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        if let Ok(mut buffer) = display_buffer().lock() {
                                                            buffer.log_error(format!("Project generation failed: {}", e));
                                                        }
                                                    }
                                                }
                                            } else if let Ok(mut buffer) = display_buffer().lock() {
                                                buffer.log_info(format!("Project {} already exists", joined_str));
                                            }

                                            if *vscode_open_enabled.read() {
                                                let _ = cmd::start_rd();
                                                if let Ok(mut buffer) = display_buffer().lock() {
                                                    buffer.log_info(format!("Visual Studio Code opened: {}", joined_str));
                                                }
                                                if let Err(e) = cmd::open_vscode(&joined_str) {
                                                    if let Ok(mut buffer) = display_buffer().lock() {
                                                        buffer.log_error(format!("Visual Studio Code failed to open: {}", e));
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "Create"
                                }
                            }

                            div {
                                class: "checkbox-row",
                                input {
                                    r#type: "checkbox",
                                    checked: *vscode_open_enabled.read(),
                                    onchange: move |ev| {
                                        vscode_open_enabled.set(ev.checked());
                                    }
                                }
                                span { class: "checkbox-label", "Open VS Code after creation" }
                            }

                            a {
                                class: "template-link",
                                href: "{parameter::TEMPLATE_URL}",
                                "View template repository"
                            }
                        }
                    }

                    // ---- DAP Server ----
                    section {
                        class: "card",
                        div {
                            class: "card-header-row",
                            div {
                                h2 { class: "section-title", "probe-rs DAP Server" }
                                p { class: "section-subtitle", "Launch a local DAP server for debugging." }
                            }
                            div {
                                class: "dap-status",
                                span { class: "{dap_dot_class(*dap_running.read())}" }
                                span {
                                    if *dap_running.read() { "Running" } else { "Stopped" }
                                }
                            }
                        }

                        div {
                            class: "card-body",
                            div {
                                class: "input-row",
                                label { class: "input-label", "Port" }
                                input {
                                    class: "input input-narrow",
                                    value: "{dap_port}",
                                    oninput: move |ev| {
                                        let value = ev.value();
                                        dap_port.set(value.clone());
                                        if let Ok(mut server) = dap_server().lock() {
                                            server.port = value;
                                        }
                                    }
                                }
                                button {
                                    class: "btn-primary",
                                    disabled: *dap_running.read(),
                                    onclick: move |_| {
                                        if let Ok(mut server) = dap_server().lock() {
                                            let tx = {
                                                if let Ok(buffer) = display_buffer().lock() {
                                                    buffer.sender()
                                                } else {
                                                    return;
                                                }
                                            };
                                            match server.start(tx) {
                                                Ok(()) => {
                                                    dap_running.set(true);
                                                    if let Ok(mut buffer) = display_buffer().lock() {
                                                        buffer.log_info(format!(
                                                            "probe-rs DAP Server started on port {}",
                                                            server.port
                                                        ));
                                                    }
                                                }
                                                Err(e) => {
                                                    if let Ok(mut buffer) = display_buffer().lock() {
                                                        buffer.log_error(e.clone());
                                                    }
                                                    last_error.set(Some(e));
                                                }
                                            }
                                        }
                                    },
                                    "Run"
                                }
                                button {
                                    class: "btn-danger",
                                    disabled: !*dap_running.read(),
                                    onclick: move |_| {
                                        if let Ok(mut server) = dap_server().lock() {
                                            if server.stop() {
                                                dap_running.set(false);
                                                if let Ok(mut buffer) = display_buffer().lock() {
                                                    buffer.log_info("probe-rs DAP Server stopped".to_string());
                                                }
                                            }
                                        }
                                    },
                                    "Stop"
                                }
                            }
                            p {
                                class: "hint-text",
                                "Docker path mapping: set pathMappings in launch.json (remoteRoot / localRoot)."
                            }
                        }
                    }
                }

                // ---- Log (full width below grid) ----
                section {
                    class: "card card-full",
                    div {
                        class: "card-header-row",
                        div {
                            h2 { class: "section-title", "Log" }
                            p { class: "section-subtitle", "Build and runtime output." }
                        }
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let text = logs.read().join("\n");
                                let escaped = escape_js_string(&text);
                                document::eval(&format!(
                                    "navigator.clipboard.writeText(\"{}\")",
                                    escaped
                                ));
                            },
                            "Copy"
                        }
                    }
                    div {
                        class: "log-viewer",
                        for (idx, line) in logs.read().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "log-line {log_level_class(line)}",
                                span { class: "log-timestamp", "{extract_timestamp(line)}" }
                                span { class: "log-badge {log_badge_class(line)}", "{extract_level(line)}" }
                                span { class: "log-message", "{extract_message(line)}" }
                            }
                        }
                    }
                }
            }

            // ---- Error toast ----
            if let Some(err) = last_error.read().clone() {
                div {
                    class: "error-toast",
                    div {
                        class: "error-toast-inner",
                        span { "[ERROR] {err}" }
                        button {
                            class: "error-dismiss",
                            onclick: move |_| last_error.set(None),
                            "\u{00d7}"
                        }
                    }
                }
            }

            // ---- Docker startup prompt ----
            if docker_status.read().contains("Off") && !*docker_prompt_dismissed.read() && !*show_splash.read() {
                div {
                    class: "modal-overlay",
                    div {
                        class: "modal",
                        h3 { class: "modal-title", "Rancher Desktop" }
                        p { class: "modal-text", "Docker is not running." }
                        p { class: "modal-text", "Start Rancher Desktop now?" }
                        div {
                            class: "modal-actions",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    match cmd::start_rd() {
                                        Ok(_) => {
                                            if let Ok(mut buffer) = display_buffer().lock() {
                                                buffer.log_info("Rancher Desktop started".to_string());
                                            }
                                        }
                                        Err(e) => {
                                            if let Ok(mut buffer) = display_buffer().lock() {
                                                buffer.log_error(format!("Rancher Desktop start failed: {}", e));
                                            }
                                            last_error.set(Some(e));
                                        }
                                    }
                                    docker_prompt_dismissed.set(true);
                                },
                                "Start"
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| docker_prompt_dismissed.set(true),
                                "Not now"
                            }
                        }
                    }
                }
            }

            // ---- Splash screen (once per day) ----
            if *show_splash.read() {
                div {
                    class: "splash-overlay",
                    onclick: move |_| show_splash.set(false),
                    div { class: "splash-glow" }
                    div { class: "splash-glow splash-glow-2" }
                    div {
                        class: "splash-content",
                        img {
                            class: "splash-logo",
                            src: "{logo_data_uri()}",
                            alt: "Baker Link",
                        }
                        div { class: "splash-shimmer" }
                    }
                    p { class: "splash-subtitle", "{parameter::APP_NAME}" }
                    p { class: "splash-version", "{parameter::build_version_label()}" }
                }
            }
        }
    }
}

// ---- Helper functions ----

fn load_window_icon() -> Icon {
    let bytes = include_bytes!("../icon/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).expect("Failed to create icon")
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn docker_dot_class(status: &str) -> &'static str {
    if status.contains("On") {
        "status-dot status-dot-green"
    } else if status.contains("Off") {
        "status-dot status-dot-red"
    } else {
        "status-dot status-dot-gray"
    }
}

fn dap_dot_class(running: bool) -> &'static str {
    if running {
        "status-dot status-dot-pulse"
    } else {
        "status-dot status-dot-gray"
    }
}

fn log_level_class(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "log-error"
    } else if line.contains("[INFO]") {
        "log-info"
    } else {
        "log-debug"
    }
}

fn log_badge_class(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "log-badge-error"
    } else if line.contains("[INFO]") {
        "log-badge-info"
    } else {
        "log-badge-default"
    }
}

fn extract_level(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "ERR"
    } else if line.contains("[INFO]") {
        "INF"
    } else {
        "LOG"
    }
}

/// Extract timestamp from log line format: "2024-01-15 12:34:56.789[LEVEL]: message"
fn extract_timestamp(line: &str) -> &str {
    if line.len() >= 23 {
        &line[..23]
    } else {
        ""
    }
}

/// Extract message after "[LEVEL]: " from log line
fn extract_message(line: &str) -> &str {
    if let Some(pos) = line.find("]: ") {
        &line[pos + 3..]
    } else if line.len() > 23 {
        &line[23..]
    } else {
        line
    }
}

// ---- History persistence ----

fn history_file_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&appdata)
            .join("baker-link-env")
            .join("history.json")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&home)
            .join(".config")
            .join("baker-link-env")
            .join("history.json")
    }
}

fn load_history() -> Vec<HistoryEntry> {
    let path = history_file_path();
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_history(entries: &[HistoryEntry]) {
    let path = history_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(&path, data);
    }
}

// ---- Splash screen (once per day) ----

fn splash_file_path() -> std::path::PathBuf {
    let parent = history_file_path();
    parent.with_file_name("last_splash.txt")
}

fn should_show_splash() -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match std::fs::read_to_string(splash_file_path()) {
        Ok(date) => date.trim() != today,
        Err(_) => true,
    }
}

fn mark_splash_shown() {
    let path = splash_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _ = std::fs::write(path, today);
}
