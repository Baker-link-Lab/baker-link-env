#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread;

use probe_rs_tools::cmd::dap_server;
use time::UtcOffset;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;

use crate::parameter;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "macos")]
const ZSH_PROFILE: &str = ".zshrc";

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Project {
    pub name: String,
    path: String,
}

impl Project {
    pub fn get_path(&self) -> String {
        self.joined_path().to_str().unwrap().to_string()
    }

    pub fn is_folder_exists(&self) -> bool {
        std::path::Path::new(&self.get_path()).exists()
    }

    fn joined_path(&self) -> std::path::PathBuf {
        std::path::Path::new(&self.path).join(&self.name)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct NewProject {
    pub name: String,
    pub path: String,
    pub vscode_open_enabled: bool,
    pub history: Vec<Project>,
    history_max: usize,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ProbeRsDapServer {
    pub port: String,
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

impl Default for NewProject {
    fn default() -> Self {
        Self {
            name: "myproject".to_string(),
            path: "".to_string(),
            vscode_open_enabled: true,
            history: Vec::with_capacity(10),
            history_max: 10,
        }
    }
}

impl Default for ProbeRsDapServer {
    fn default() -> Self {
        Self {
            port: 50001.to_string(),
            shutdown: None,
            handle: None,
            status: DapServerStatus::Stopped,
        }
    }
}

impl NewProject {
    pub fn history_push(&mut self) -> bool {
        let current = Project {
            name: self.name.clone(),
            path: self.path.clone(),
        };

        if self.history.contains(&current) {
            return false;
        }

        if self.history.len() == self.history_max {
            self.history.remove(0);
        }

        self.history.push(current);
        true
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
}

pub fn generate_project(name: &str, path: &str) -> anyhow::Result<std::path::PathBuf> {
    std::env::set_current_dir(path).unwrap();
    let generate_args = cargo_generate::GenerateArgs {
        name: Some(name.to_string()),
        vcs: Some(cargo_generate::Vcs::Git),
        template_path: cargo_generate::TemplatePath {
            git: Some(parameter::TUTORIAL_TEMPLATE.to_string()),
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
}

impl ProbeRsDapServer {
    pub fn start(&mut self, tx: std::sync::mpsc::Sender<String>) -> Result<(), String> {
        if self.status != DapServerStatus::Stopped {
            return Ok(());
        }
        let port = self.parse_port()?;
        let shutdown = CancellationToken::new();
        let shutdown_task = shutdown.clone();
        let log_tx = tx.clone();

        let handle = spawn_dap_server_thread(port, shutdown_task, log_tx);

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
}

fn spawn_dap_server_thread(
    port: u16,
    shutdown_task: CancellationToken,
    log_tx: std::sync::mpsc::Sender<String>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let runtime = match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = log_tx.send(format!("Failed to start runtime: {error}"));
                return;
            }
        };

        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let result = runtime.block_on(dap_server::run_with_shutdown_on_port(
            port,
            false,
            None,
            offset,
            shutdown_task,
        ));

        if let Err(error) = result {
            let _ = log_tx.send(format!("DAP server stopped: {error}"));
        }
    })
}

#[cfg(target_os = "windows")]
fn open_vscode_windows(path: &str) -> Result<std::process::Output, std::io::Error> {
    let env_path = std::env::var("PATH").unwrap();
    std::process::Command::new("code.cmd")
        .arg(path)
        .env("PATH", env_path)
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
    let path = std::env!("PATH");
    match std::process::Command::new("rdctl")
        .arg("start")
        .arg("--application.start-in-background")
        .env("PATH", path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error: {}, $PATH: {}", e, path)),
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
    let path = std::env!("PATH");
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .env("PATH", path)
        .output()
        .map_err(|e| format!("docker info failed: {}", e))?;
    Ok(output.status.success())
}

#[cfg(target_os = "macos")]
fn docker_info_macos() -> Result<bool, String> {
    let output = Command::new("docker")
        .arg("info")
        .arg("--format")
        .arg("{{.ServerVersion}}")
        .output()
        .map_err(|e| format!("docker info failed: {}", e))?;
    Ok(output.status.success())
}

pub fn are_apps_running(app_name: &str) -> bool {
    let mut system = sysinfo::System::new_all();
    system.refresh_processes();
    let count = system
        .processes()
        .values()
        .filter(|p| p.name() == app_name)
        .count();
    count >= 2
}
