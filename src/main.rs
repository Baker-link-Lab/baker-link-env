#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cmd;
mod logger;
mod parameter;

use dioxus::prelude::*;
use log::LevelFilter;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

static DISPLAY_BUFFER: OnceLock<Mutex<logger::DisplayBuffer>> = OnceLock::new();
static DAP_SERVER: OnceLock<Mutex<cmd::ProbeRsDapServer>> = OnceLock::new();

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
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut project_name = use_signal(|| "myproject".to_string());
    let mut vscode_open_enabled = use_signal(|| true);
    let mut dap_port = use_signal(|| "50001".to_string());
    let mut dap_running = use_signal(|| false);
    let mut logs = use_signal(Vec::<String>::new);
    let mut docker_status = use_signal(|| "Docker: ?".to_string());
    let mut last_error = use_signal(|| Option::<String>::None);
    let mut log_tick = use_signal(|| 0_u64);

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

    rsx! {
        document::Title { "{parameter::APP_NAME}" }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

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

                // Right: Help
                div {
                    class: "top-bar-right",
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
                    h2 { class: "section-title", "Log" }
                    p { class: "section-subtitle", "Build and runtime output." }
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
        }
    }
}

// ---- Helper functions ----

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

#[allow(dead_code)]
fn _start_log_auto_polling() {
    thread::spawn(move || loop {
        if let Ok(mut buffer) = display_buffer().lock() {
            buffer.channel_recv();
        }
        thread::sleep(Duration::from_millis(300));
    });
}
