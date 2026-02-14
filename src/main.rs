#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cmd;
mod logger;
mod parameter;
mod ui_modules;
mod uiutil;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../icon/icon.png")[..])
                    .expect("Failed to load icon"),
            )
            .with_inner_size([480.0, 340.0])
            .with_min_inner_size([360.0, 260.0])
            .with_max_inner_size([560.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        parameter::APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(app::EvnApp::new(cc)))),
    )
}
