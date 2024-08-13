#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cmd;
mod logger;
mod parameter;
mod uiutil;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let _ = cmd::start_rd();

    if cmd::are_apps_runnning("baker-link-env") {
        println!("baker-link-env is already running.");
        return Ok(());
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../icon/icon.png")[..])
                    .expect("Failed to load icon"),
            )
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        parameter::APP_NAME,
        native_options,
        Box::new(|cc| {
            uiutil::set_fonts(cc);
            uiutil::set_background_color(cc, egui::Color32::from_hex("#fff9ee").unwrap());
            Ok(Box::new(app::EvnApp::new(cc)))
        }),
    )
}
