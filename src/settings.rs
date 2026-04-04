use std::path::PathBuf;

const APP_DIR_NAME: &str = "baker-link-env";
const SETTINGS_FILE: &str = "settings.json";

pub const HISTORY_MAX: usize = 10;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    #[serde(default)]
    pub last_splash_date: String,
}

fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
        .join(SETTINGS_FILE)
}

pub fn load() -> AppSettings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

pub fn save(settings: &AppSettings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, data);
    }
}

pub fn load_history() -> Vec<HistoryEntry> {
    load().history
}

pub fn save_history(entries: &[HistoryEntry]) {
    let mut s = load();
    s.history = entries.to_vec();
    save(&s);
}

pub fn should_show_splash() -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    load().last_splash_date != today
}

pub fn mark_splash_shown() {
    let mut s = load();
    s.last_splash_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    save(&s);
}

pub fn reset_all() {
    let path = settings_path();
    let _ = std::fs::remove_file(path);
}
