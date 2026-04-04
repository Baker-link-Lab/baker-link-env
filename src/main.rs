#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cmd;
mod helpers;
mod logger;
mod parameter;
mod settings;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use log::LevelFilter;
use std::sync::{Mutex, OnceLock};

const INLINE_CSS: &str = include_str!("../assets/main.css");
const LOGO_PNG_BYTES: &[u8] = include_bytes!("../assets/baker-link-logo.png");

static DISPLAY_BUFFER: OnceLock<Mutex<logger::DisplayBuffer>> = OnceLock::new();
static DAP_SERVER: OnceLock<Mutex<cmd::ProbeRsDapServer>> = OnceLock::new();

pub fn display_buffer() -> &'static Mutex<logger::DisplayBuffer> {
    DISPLAY_BUFFER.get_or_init(|| Mutex::new(logger::DisplayBuffer::new()))
}

pub fn dap_server() -> &'static Mutex<cmd::ProbeRsDapServer> {
    DAP_SERVER.get_or_init(|| Mutex::new(cmd::ProbeRsDapServer::default()))
}

pub fn log_info(msg: impl Into<String>) {
    if let Ok(mut buf) = display_buffer().lock() {
        buf.log_info(msg.into());
    }
}

pub fn log_error(msg: impl Into<String>) {
    if let Ok(mut buf) = display_buffer().lock() {
        buf.log_error(msg.into());
    }
}

fn main() {
    let mut logger = env_logger::Builder::new();
    if std::env::var("RUST_LOG").is_ok() {
        logger.parse_default_env();
    } else {
        logger.filter_level(LevelFilter::Off);
    }
    let _ = logger.try_init();

    let icon = helpers::load_window_icon();
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_icon(icon).with_window(
                WindowBuilder::new()
                    .with_title(parameter::APP_NAME)
                    .with_inner_size(LogicalSize::new(960.0, 680.0))
                    .with_min_inner_size(LogicalSize::new(800.0, 560.0)),
            ),
        )
        .launch(app::App);
}
