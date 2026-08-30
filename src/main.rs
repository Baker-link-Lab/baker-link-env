#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod artifact;
mod cmd;
mod helpers;
mod logger;
mod parameter;
mod settings;
mod upload;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use log::LevelFilter;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const LOGO_PNG_BYTES: &[u8] = include_bytes!("../assets/baker-link-logo.png");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

static DAP_SERVER: OnceLock<Mutex<cmd::ProbeRsDapServer>> = OnceLock::new();

pub use logger::{display_buffer, log_error, log_info};

pub fn dap_server() -> &'static Mutex<cmd::ProbeRsDapServer> {
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

    if let Err(error) = settings::validate() {
        eprintln!("Failed to load settings: {error}");
        std::process::exit(1);
    }

    if std::env::args().any(|argument| argument == "--headless") {
        if let Err(error) = run_headless() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    let icon = helpers::load_window_icon();
    let custom_head = format!(r#"<link rel="stylesheet" href="{TAILWIND_CSS}">"#);
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_icon(icon)
                .with_custom_head(custom_head)
                .with_background_color((0x11, 0x10, 0x14, 0xff))
                .with_window(
                    WindowBuilder::new()
                        .with_title(parameter::APP_NAME)
                        .with_inner_size(LogicalSize::new(960.0, 680.0))
                        .with_min_inner_size(LogicalSize::new(800.0, 560.0)),
                ),
        )
        .launch(app::App);
}

fn run_headless() -> Result<(), String> {
    let config = settings::server_config().map_err(|error| error.to_string())?;
    let (log_tx, log_rx) = std::sync::mpsc::channel();
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.send(());
    })
    .map_err(|error| format!("Failed to install shutdown handler: {error}"))?;

    {
        let mut server = dap_server()
            .lock()
            .map_err(|_| "DAP server lock is poisoned".to_string())?;
        server.ip = config.bind_ip.clone();
        server.port = config.dap_port.clone();
        server.upload_port = config.upload_port.clone();
        server.start(log_tx)?;
    }
    for entry in settings::load_history().map_err(|error| error.to_string())? {
        if let Err(error) =
            cmd::prepare_project_connection(std::path::Path::new(&entry.path), &config)
        {
            eprintln!("Failed to refresh connection for {}: {error}", entry.name);
        }
    }
    println!(
        "Baker Link Env listening on {} (DAP {}) and {} (ELF upload)",
        config.bind_ip, config.dap_port, config.upload_port
    );

    loop {
        while let Ok(message) = log_rx.try_recv() {
            println!("{message}");
        }
        let stopped = dap_server()
            .lock()
            .map_err(|_| "DAP server lock is poisoned".to_string())?
            .refresh_status()
            == cmd::DapServerStatus::Stopped;
        if stopped {
            return Err("Baker Link Env server stopped unexpectedly".to_string());
        }
        if shutdown_rx.recv_timeout(Duration::from_millis(100)).is_ok() {
            break;
        }
    }

    let mut server = dap_server()
        .lock()
        .map_err(|_| "DAP server lock is poisoned".to_string())?;
    server.stop();
    while server.refresh_status() != cmd::DapServerStatus::Stopped {
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("Baker Link Env stopped");
    Ok(())
}
