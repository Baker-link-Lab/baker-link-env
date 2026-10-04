use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_DIR_NAME: &str = "baker-link-env";
const SETTINGS_FILE: &str = "settings.json";

pub const HISTORY_MAX: usize = 10;

#[derive(Clone, Default)]
pub struct ServerConfig {
    pub bind_ip: String,
    pub connect_host: String,
    pub dap_port: String,
    pub upload_port: String,
}

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
    #[serde(default = "default_bind_ip")]
    pub bind_ip: String,
    #[serde(default = "default_connect_host")]
    pub connect_host: String,
    #[serde(default = "default_dap_port")]
    pub dap_port: String,
    #[serde(default = "default_upload_port")]
    pub upload_port: String,
}

fn default_bind_ip() -> String {
    "0.0.0.0".to_string()
}

fn default_connect_host() -> String {
    "host.docker.internal".to_string()
}

fn default_dap_port() -> String {
    "50001".to_string()
}

fn default_upload_port() -> String {
    "50002".to_string()
}

fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
        .join(SETTINGS_FILE)
}

pub fn load() -> Result<AppSettings> {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(error).with_context(|| format!("Failed to read {}", path.display())),
    }
}

pub fn validate() -> Result<()> {
    load().map(|_| ())
}

pub fn save(settings: &AppSettings) -> Result<()> {
    let path = settings_path();
    let parent = path
        .parent()
        .context("Settings path has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create {}", parent.display()))?;
    let data = serde_json::to_vec_pretty(settings)?;
    let temp_path = parent.join(format!(".{SETTINGS_FILE}.{}.tmp", std::process::id()));
    std::fs::write(&temp_path, data)
        .with_context(|| format!("Failed to write {}", temp_path.display()))?;
    std::fs::rename(&temp_path, &path).with_context(|| {
        format!(
            "Failed to replace {} with {}",
            path.display(),
            temp_path.display()
        )
    })?;
    Ok(())
}

pub fn server_config() -> Result<ServerConfig> {
    let settings = load()?;
    Ok(ServerConfig {
        bind_ip: if settings.bind_ip.is_empty() {
            default_bind_ip()
        } else {
            settings.bind_ip
        },
        connect_host: if settings.connect_host.is_empty() {
            default_connect_host()
        } else {
            settings.connect_host
        },
        dap_port: if settings.dap_port.is_empty() {
            default_dap_port()
        } else {
            settings.dap_port
        },
        upload_port: if settings.upload_port.is_empty() {
            default_upload_port()
        } else {
            settings.upload_port
        },
    })
}

pub fn save_server_config(config: &ServerConfig) -> Result<()> {
    let mut settings = load()?;
    settings.bind_ip = config.bind_ip.clone();
    settings.connect_host = config.connect_host.clone();
    settings.dap_port = config.dap_port.clone();
    settings.upload_port = config.upload_port.clone();
    save(&settings)
}

pub fn load_history() -> Result<Vec<HistoryEntry>> {
    Ok(load()?.history)
}

pub fn save_history(entries: &[HistoryEntry]) -> Result<()> {
    let mut s = load()?;
    s.history = entries.to_vec();
    save(&s)
}

pub fn should_show_splash() -> Result<bool> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    Ok(load()?.last_splash_date != today)
}

pub fn mark_splash_shown() -> Result<()> {
    let mut s = load()?;
    s.last_splash_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    save(&s)
}

pub fn reset_all() -> Result<()> {
    let path = settings_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Failed to remove {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_settings_allow_container_connections() {
        let settings: AppSettings = serde_json::from_str(r#"{"history":[]}"#).unwrap();
        assert_eq!(settings.bind_ip, "0.0.0.0");
        assert_eq!(settings.bind_ip, crate::cmd::ProbeRsDapServer::default().ip);
        assert_eq!(settings.connect_host, "host.docker.internal");
        assert_eq!(settings.dap_port, "50001");
        assert_eq!(settings.upload_port, "50002");
    }

    #[test]
    fn preserves_explicit_loopback_binding() {
        let settings: AppSettings = serde_json::from_str(r#"{"bind_ip":"127.0.0.1"}"#).unwrap();
        assert_eq!(settings.bind_ip, "127.0.0.1");
    }
}
