#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod app;
mod cmd;
mod parameter;
mod uiutil;
mod logger;
mod infoui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_inner_size([400.0, 300.0])
            // .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../icon/icon.png")[..])
                    .expect("Failed to load icon"),
            ),
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
