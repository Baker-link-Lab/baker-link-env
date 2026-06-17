#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::net::IpAddr;
use std::process::Command;
use std::thread;

use probe_rs::config::TargetSelector;
use probe_rs::probe::list::Lister;
use probe_rs::Permissions;
use probe_rs_tools::cmd::dap_server;
use time::UtcOffset;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;

use crate::parameter;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "macos")]
const ZSH_PROFILE: &str = ".zshrc";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ProbeRsDapServer {
    pub port: String,
    pub ip: String,
    #[serde(skip)]
    shutdown: Option<CancellationToken>,
    #[serde(skip)]
    handle: Option<std::thread::JoinHandle<()>>,
    #[serde(skip)]
    pub status: DapServerStatus,
}

#[derive(PartialEq)]
pub enum DapServerStatus {
    Running(u16),
    Stopped,
}

impl Default for DapServerStatus {
    fn default() -> Self {
        DapServerStatus::Stopped
    }
}

impl Default for ProbeRsDapServer {
    fn default() -> Self {
        Self {
            port: 50001.to_string(),
            ip: "127.0.0.1".to_string(),
            shutdown: None,
            handle: None,
            status: DapServerStatus::Stopped,
        }
    }
}

pub fn open_vscode(path: &str) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        open_vscode_windows(path)
    }
    #[cfg(target_os = "macos")]
    {
        open_vscode_macos(path)
    }
    #[cfg(target_os = "linux")]
    {
        open_vscode_linux(path)
    }
}

pub fn start_rd() -> std::result::Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        start_rd_windows()
    }
    #[cfg(target_os = "macos")]
    {
        start_rd_macos()
    }
    #[cfg(target_os = "linux")]
    {
        start_rd_linux()
    }
}

pub fn generate_project(
    name: &str,
    path: &str,
    branch: Option<String>,
    tag: Option<String>,
) -> anyhow::Result<std::path::PathBuf> {
    std::env::set_current_dir(path).unwrap();
    let generate_args = cargo_generate::GenerateArgs {
        name: Some(name.to_string()),
        vcs: Some(cargo_generate::Vcs::Git),
        template_path: cargo_generate::TemplatePath {
            git: Some(parameter::TUTORIAL_TEMPLATE.to_string()),
            branch,
            tag,
            ..cargo_generate::TemplatePath::default()
        },
        ..cargo_generate::GenerateArgs::default()
    };
    cargo_generate::generate(generate_args)
}

pub fn is_docker_running() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        docker_info_windows()
    }
    #[cfg(target_os = "macos")]
    {
        docker_info_macos()
    }
    #[cfg(target_os = "linux")]
    {
        docker_info_linux()
    }
}

impl ProbeRsDapServer {
    pub fn start(&mut self, tx: std::sync::mpsc::Sender<String>) -> Result<(), String> {
        if self.status != DapServerStatus::Stopped {
            return Ok(());
        }
        let port = self.parse_port()?;
        let ip = self.parse_ip()?;
        let shutdown = CancellationToken::new();
        let shutdown_task = shutdown.clone();
        let log_tx = tx.clone();

        let handle = spawn_dap_server_thread(port, ip, shutdown_task, log_tx);

        self.shutdown = Some(shutdown);
        self.handle = Some(handle);
        self.status = DapServerStatus::Running(port);
        Ok(())
    }

    pub fn stop(&mut self) -> bool {
        if self.status == DapServerStatus::Stopped {
            return false;
        }
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.cancel();
        }
        if let Some(handle) = self.handle.take() {
            // Detach the join to avoid blocking the UI thread while the server shuts down.
            thread::spawn(move || {
                let _ = handle.join();
            });
        }
        self.status = DapServerStatus::Stopped;
        true
    }

    fn parse_port(&self) -> Result<u16, String> {
        self.port
            .parse::<u16>()
            .map_err(|_| "Invalid port number".to_string())
    }

    fn parse_ip(&self) -> Result<IpAddr, String> {
        self.ip
            .parse::<IpAddr>()
            .map_err(|_| "Invalid IP address".to_string())
    }
}

fn spawn_dap_server_thread(
    port: u16,
    ip: IpAddr,
    shutdown_task: CancellationToken,
    log_tx: std::sync::mpsc::Sender<String>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let shutdown_probe = shutdown_task.clone();
        let runtime = match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = log_tx.send(format!("Failed to start runtime: {error}"));
                return;
            }
        };

        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let result = runtime.block_on(dap_server::run_with_shutdown_on_addr(
            port,
            ip,
            false,
            None,
            offset,
            shutdown_task,
        ));

        if let Err(error) = result {
            if shutdown_probe.is_cancelled() {
                let _ = log_tx.send("DAP server shutdown requested".to_string());
            } else {
                let _ = log_tx.send(format!("[ERROR] DAP server stopped: {error}"));
            }
        }
    })
}

#[cfg(target_os = "windows")]
fn open_vscode_windows(path: &str) -> Result<std::process::Output, std::io::Error> {
    let env_path = std::env::var("PATH").unwrap();
    std::process::Command::new("code.cmd")
        .arg(path)
        .env("PATH", env_path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

#[cfg(target_os = "macos")]
fn open_vscode_macos(path: &str) -> Result<std::process::Output, std::io::Error> {
    std::process::Command::new("open")
        .arg("-a")
        .arg("Visual Studio Code")
        .arg(path)
        .output()
}

#[cfg(target_os = "windows")]
fn start_rd_windows() -> std::result::Result<(), String> {
    let path = std::env::var("PATH").unwrap_or_default();
    match std::process::Command::new("rdctl")
        .arg("start")
        .arg("--application.start-in-background")
        .env("PATH", &path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn start_rd_macos() -> std::result::Result<(), String> {
    let home_dir = std::env::var("HOME").unwrap();
    let zshrc_path = format!("{}/{}", home_dir, ZSH_PROFILE);

    match std::process::Command::new("zsh")
        .arg("-c")
        .arg(format!(
            "source {} && rdctl start --application.start-in-background",
            zshrc_path
        ))
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error: {}", e)),
    }
}

#[cfg(target_os = "windows")]
fn docker_info_windows() -> Result<bool, String> {
    let path = std::env::var("PATH").unwrap_or_default();
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .env("PATH", path)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("docker info failed: {}", e))?;
    Ok(output.status.success())
}

#[cfg(target_os = "macos")]
fn docker_info_macos() -> Result<bool, String> {
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let full_path = format!(
        "{}/.rd/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:{}",
        home_dir, current_path
    );
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .env("PATH", full_path)
        .output()
        .map_err(|e| format!("docker info failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(output.status.success() && !stdout.trim().is_empty())
}

pub struct ProbeInfo {
    pub probe_type: String,
    pub identifier: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
}

pub struct TargetInfo {
    pub probe: ProbeInfo,
    pub chip_name: String,
    pub cores: Vec<String>,
    pub target_voltage: Option<f32>,
}

pub fn list_probes() -> Vec<ProbeInfo> {
    Lister::new()
        .list_all()
        .into_iter()
        .map(|p| ProbeInfo {
            probe_type: p.probe_type(),
            identifier: p.identifier.clone(),
            vendor_id: p.vendor_id,
            product_id: p.product_id,
            serial_number: p.serial_number.clone(),
        })
        .collect()
}

pub fn detect_target() -> Result<TargetInfo, String> {
    let lister = Lister::new();
    let probes = lister.list_all();
    let probe_info_raw = probes.first().ok_or("No debug probe found".to_string())?;

    let probe_info = ProbeInfo {
        probe_type: probe_info_raw.probe_type(),
        identifier: probe_info_raw.identifier.clone(),
        vendor_id: probe_info_raw.vendor_id,
        product_id: probe_info_raw.product_id,
        serial_number: probe_info_raw.serial_number.clone(),
    };

    let probe = probe_info_raw
        .open()
        .map_err(|e| format!("Failed to open probe: {e}"))?;

    let session = probe
        .attach(TargetSelector::Auto, Permissions::default())
        .map_err(|e| format!("Failed to detect target: {e}"))?;

    let chip_name = session.target().name.clone();
    let cores: Vec<String> = session
        .list_cores()
        .into_iter()
        .map(|(_, core_type)| format!("{core_type:?}"))
        .collect();

    Ok(TargetInfo {
        probe: probe_info,
        chip_name,
        cores,
        target_voltage: None,
    })
}

#[cfg(target_os = "linux")]
fn linux_path_with_rd() -> String {
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let current_path = std::env::var("PATH").unwrap_or_default();
    format!(
        "{}/.rd/bin:/usr/local/bin:/usr/bin:/bin:{}",
        home_dir, current_path
    )
}

#[cfg(target_os = "linux")]
fn open_vscode_linux(path: &str) -> Result<std::process::Output, std::io::Error> {
    std::process::Command::new("code").arg(path).output()
}

#[cfg(target_os = "linux")]
fn start_rd_linux() -> std::result::Result<(), String> {
    match std::process::Command::new("rdctl")
        .arg("start")
        .arg("--application.start-in-background")
        .env("PATH", linux_path_with_rd())
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error: {}", e)),
    }
}

#[cfg(target_os = "linux")]
fn docker_info_linux() -> Result<bool, String> {
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .env("PATH", linux_path_with_rd())
        .output()
        .map_err(|e| format!("docker info failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(output.status.success() && !stdout.trim().is_empty())
}
