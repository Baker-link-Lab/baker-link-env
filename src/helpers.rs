use dioxus::desktop::tao::window::Icon;

use base64::Engine as _;

pub fn load_window_icon() -> Icon {
    let bytes = include_bytes!("../icon/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).expect("Failed to create icon")
}

pub fn logo_data_uri() -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(crate::LOGO_PNG_BYTES);
    format!("data:image/png;base64,{b64}")
}

pub fn docker_dot_class(status: &str) -> &'static str {
    if status.contains("On") {
        "status-dot status-dot-green"
    } else if status.contains("Off") {
        "status-dot status-dot-red"
    } else {
        "status-dot status-dot-gray"
    }
}

pub fn dap_dot_class(running: bool) -> &'static str {
    if running {
        "status-dot status-dot-pulse"
    } else {
        "status-dot status-dot-gray"
    }
}
