#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod artifact;
mod cmd;
mod doctor;
mod helpers;
mod host_network;
mod logger;
mod parameter;
mod settings;
mod upload;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use log::LevelFilter;
use std::sync::{Mutex, OnceLock};

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
