use anyhow::{bail, Context};
use std::net::{IpAddr, SocketAddr, TcpListener};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::Path;
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
    pub upload_port: String,
    pub ip: String,
    #[serde(skip)]
    shutdown: Option<CancellationToken>,
    #[serde(skip)]
    handle: Option<std::thread::JoinHandle<()>>,
    #[serde(skip)]
    completion: Option<std::sync::mpsc::Receiver<()>>,
    #[serde(skip)]
    pub status: DapServerStatus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DapServerStatus {
    Running(u16),
    Stopping,
    #[default]
    Stopped,
}

impl Default for ProbeRsDapServer {
    fn default() -> Self {
        Self {
            port: 50001.to_string(),
            upload_port: crate::upload::DEFAULT_UPLOAD_PORT.to_string(),
            ip: "0.0.0.0".to_string(),
            shutdown: None,
            handle: None,
            completion: None,
            status: DapServerStatus::Stopped,
        }
    }
}

pub fn open_vscode(path: &str) -> anyhow::Result<()> {
    let path = std::path::absolute(path)?;
    if !path.is_dir() {
        bail!("Project folder does not exist: {}", path.display());
    }
    let container_uri = dev_container_uri(&path)?;
    let arguments = match container_uri {
        Some(uri) => {
            let extensions = run_vscode(&["--list-extensions".to_string()])?;
            if !String::from_utf8_lossy(&extensions.stdout)
                .lines()
                .any(|line| {
                    line.trim()
                        .eq_ignore_ascii_case("ms-vscode-remote.remote-containers")
                })
            {
                bail!("Install the Dev Containers extension (ms-vscode-remote.remote-containers) in VS Code");
            }
            wait_for_docker()?;
            vec!["--new-window".to_string(), "--folder-uri".to_string(), uri]
        }
        None => vec![path
            .to_str()
            .context("Project path is not valid UTF-8")?
            .to_string()],
    };
    run_vscode(&arguments)?;
    Ok(())
}

fn dev_container_uri(path: &Path) -> anyhow::Result<Option<String>> {
    for relative in [".devcontainer/devcontainer.json", ".devcontainer.json"] {
        let config_path = path.join(relative);
        match std::fs::read_to_string(&config_path) {
            Ok(config) => {
                return build_dev_container_uri(
                    path.to_str().context("Project path is not valid UTF-8")?,
                    &config,
                )
                .with_context(|| format!("Failed to read {}", config_path.display()))
                .map(Some);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to read {}", config_path.display()));
            }
        }
    }
    Ok(None)
}

fn build_dev_container_uri(host_path: &str, config: &str) -> anyhow::Result<String> {
    let config: serde_json::Value = json5::from_str(config)?;
    let workspace = config["workspaceFolder"].as_str().context(
        "Set an explicit workspaceFolder in devcontainer.json to open directly in a container",
    )?;
    let basename = host_path
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .context("Project path has no folder name")?;
    let workspace = workspace.replace("${localWorkspaceFolderBasename}", basename);
    if !workspace.starts_with('/') || workspace.contains("${") {
        bail!("workspaceFolder must be an absolute container path; only ${{localWorkspaceFolderBasename}} substitution is supported");
    }
    // Dev Containers encodes the host folder as UTF-8 hex in the remote authority.
    let authority: String = host_path
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut uri = url::Url::parse(&format!("vscode-remote://dev-container+{authority}/"))?;
    uri.set_path(&workspace.replace('%', "%25"));
    Ok(uri.to_string())
}

fn wait_for_docker() -> anyhow::Result<()> {
    if is_docker_running().map_err(anyhow::Error::msg)? {
        return Ok(());
    }
    crate::log_info("Starting Rancher Desktop; waiting for Docker...");
    start_rd().map_err(anyhow::Error::msg)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        if is_docker_running().map_err(anyhow::Error::msg)? {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!("Docker did not become ready within 60 seconds. Start Rancher Desktop and try again");
        }
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn run_vscode(arguments: &[String]) -> anyhow::Result<std::process::Output> {
    let output = vscode_command()
        .args(arguments)
        .output()
        .context("Failed to execute VS Code CLI; ensure Visual Studio Code is installed and its code command is available")?;
    if !output.status.success() {
        bail!(
            "VS Code CLI failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output)
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
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(path)?;
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
    let result = cargo_generate::generate(generate_args);
    let restore_result = std::env::set_current_dir(original_dir);
    let project_path = result?;
    restore_result?;
    Ok(project_path)
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
        self.refresh_status();
        if self.status != DapServerStatus::Stopped {
            return Err(format!("DAP server is currently {:?}", self.status));
        }
        let port = self.parse_port()?;
        let upload_port = self.parse_upload_port()?;
        let ip = self.parse_ip()?;
        let address = SocketAddr::new(ip, port);
        let upload_address = SocketAddr::new(ip, upload_port);
        if port == upload_port {
            return Err("DAP port and ELF upload port must be different".to_string());
        }
        TcpListener::bind(address)
            .map_err(|error| format!("DAP port {port} is unavailable: {error}"))?;
        TcpListener::bind(upload_address)
            .map_err(|error| format!("ELF upload port {upload_port} is unavailable: {error}"))?;

        let artifact_root = crate::artifact::root_dir().map_err(|error| error.to_string())?;
        std::env::set_var(crate::artifact::ARTIFACT_ROOT_ENV, &artifact_root);
        let shutdown = CancellationToken::new();
        let shutdown_task = shutdown.clone();
        let log_tx = tx.clone();
        let (completion_tx, completion_rx) = std::sync::mpsc::channel();

        let handle = spawn_dap_server_thread(
            address,
            upload_address,
            artifact_root,
            shutdown_task,
            log_tx,
            completion_tx,
        );

        self.shutdown = Some(shutdown);
        self.handle = Some(handle);
        self.completion = Some(completion_rx);
        self.status = DapServerStatus::Running(port);
        Ok(())
    }

    pub fn stop(&mut self) -> bool {
        self.refresh_status();
        if matches!(
            self.status,
            DapServerStatus::Stopped | DapServerStatus::Stopping
        ) {
            return false;
        }
        self.status = DapServerStatus::Stopping;
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.cancel();
        }
        true
    }

    pub fn refresh_status(&mut self) -> DapServerStatus {
        let completed = self
            .completion
            .as_ref()
            .is_some_and(|receiver| receiver.try_recv().is_ok());
        if completed {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
            self.completion = None;
            self.shutdown = None;
            self.status = DapServerStatus::Stopped;
        }
        self.status
    }

    fn parse_port(&self) -> Result<u16, String> {
        self.port
            .parse::<u16>()
            .map_err(|_| "Invalid port number".to_string())
    }

    fn parse_upload_port(&self) -> Result<u16, String> {
        self.upload_port
            .parse::<u16>()
            .map_err(|_| "Invalid ELF upload port number".to_string())
    }

    fn parse_ip(&self) -> Result<IpAddr, String> {
        self.ip
            .parse::<IpAddr>()
            .map_err(|_| "Invalid IP address".to_string())
    }
}

fn spawn_dap_server_thread(
    address: SocketAddr,
    upload_address: SocketAddr,
    artifact_root: std::path::PathBuf,
    shutdown_task: CancellationToken,
    log_tx: std::sync::mpsc::Sender<String>,
    completion_tx: std::sync::mpsc::Sender<()>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let shutdown_probe = shutdown_task.clone();
        let upload_shutdown = shutdown_task.clone();
        let upload_log = log_tx.clone();
        let upload_handle = thread::spawn(move || {
            if let Err(error) = crate::upload::run(
                upload_address,
                artifact_root,
                upload_shutdown.clone(),
                upload_log.clone(),
            ) {
                let _ = upload_log.send(format!("[ERROR] ELF upload server stopped: {error}"));
                upload_shutdown.cancel();
            }
        });
        let runtime = match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = log_tx.send(format!("Failed to start runtime: {error}"));
                shutdown_task.cancel();
                let _ = upload_handle.join();
                let _ = completion_tx.send(());
                return;
            }
        };

        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let result = runtime.block_on(dap_server::run_with_shutdown_on_addr(
            address,
            false,
            None,
            offset,
            shutdown_task.clone(),
        ));

        if let Err(error) = result {
            if shutdown_probe.is_cancelled() {
                let _ = log_tx.send("DAP server shutdown requested".to_string());
            } else {
                let _ = log_tx.send(format!("[ERROR] DAP server stopped: {error}"));
            }
        }
        shutdown_task.cancel();
        let _ = upload_handle.join();
        let _ = completion_tx.send(());
    })
}

#[cfg(target_os = "windows")]
fn vscode_command() -> Command {
    let mut command = Command::new("code.cmd");
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(target_os = "macos")]
fn vscode_command() -> Command {
    let cli = Path::new("/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code");
    if cli.is_file() {
        return Command::new(cli);
    }
    if let Some(home) = dirs::home_dir() {
        let cli = home.join("Applications/Visual Studio Code.app/Contents/Resources/app/bin/code");
        if cli.is_file() {
            return Command::new(cli);
        }
    }
    Command::new("code")
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
}

pub struct TargetInfo {
    pub probe: ProbeInfo,
    pub chip_name: String,
    pub cores: Vec<String>,
}

pub fn list_probes() -> Vec<ProbeInfo> {
    Lister::new()
        .list_all()
        .into_iter()
        .map(|p| ProbeInfo {
            probe_type: p.probe_type(),
            identifier: p.identifier.clone(),
        })
        .collect()
}

pub fn detect_target() -> Result<TargetInfo, String> {
    let lister: Lister = Lister::new();
    let probes = lister.list_all();
    let probe_info_raw = probes.first().ok_or("No debug probe found".to_string())?;

    let probe_info = ProbeInfo {
        probe_type: probe_info_raw.probe_type(),
        identifier: probe_info_raw.identifier.clone(),
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
fn vscode_command() -> Command {
    Command::new("code")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn builds_uri_for_template_workspace() {
        let host = "/home/user/myproject";
        let uri = build_dev_container_uri(
            host,
            r#"{
                // JSONC comments and trailing commas are accepted.
                "workspaceFolder": "/myproject",
            }"#,
        )
        .unwrap();
        let encoded: String = host.bytes().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            uri,
            format!("vscode-remote://dev-container+{encoded}/myproject")
        );
    }

    #[test]
    fn encodes_windows_host_and_workspace_special_characters() {
        let host = r"C:\Users\user\my project";
        let uri = build_dev_container_uri(
            host,
            r#"{ "workspaceFolder": "/workspaces/${localWorkspaceFolderBasename}/日本語 #?%" }"#,
        )
        .unwrap();
        let encoded: String = host.bytes().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            uri,
            format!(
                "vscode-remote://dev-container+{encoded}/workspaces/my%20project/%E6%97%A5%E6%9C%AC%E8%AA%9E%20%23%3F%25"
            )
        );
    }

    #[test]
    fn rejects_ambiguous_container_workspaces() {
        for config in [
            "{}",
            r#"{ "workspaceFolder": "relative/path" }"#,
            r#"{ "workspaceFolder": "/workspaces/${localEnv:PROJECT}" }"#,
            "invalid JSON",
        ] {
            assert!(build_dev_container_uri("/home/user/project", config).is_err());
        }
    }

    #[test]
    fn detects_devcontainer_config_without_modifying_project() {
        let root = std::env::temp_dir().join(format!(
            "bakerlink-vscode-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(dev_container_uri(&root).unwrap(), None);

        let config = r#"{ "workspaceFolder": "/project" }"#;
        std::fs::write(root.join(".devcontainer.json"), config).unwrap();
        assert!(dev_container_uri(&root)
            .unwrap()
            .unwrap()
            .ends_with("/project"));
        std::fs::create_dir(root.join(".devcontainer")).unwrap();
        std::fs::write(root.join(".devcontainer/devcontainer.json"), "invalid JSON").unwrap();
        assert!(dev_container_uri(&root).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join(".devcontainer.json")).unwrap(),
            config
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    fn unused_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn wait_until_stopped(server: &mut ProbeRsDapServer) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while server.refresh_status() != DapServerStatus::Stopped {
            assert!(Instant::now() < deadline, "DAP server did not stop in time");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn stop_releases_ports_before_restart() {
        let (log_tx, _log_rx) = std::sync::mpsc::channel();
        let mut server = ProbeRsDapServer::default();
        let dap_port = unused_port();
        let upload_port = unused_port();
        server.ip = "127.0.0.1".to_string();
        server.port = dap_port.to_string();
        server.upload_port = upload_port.to_string();

        server.start(log_tx.clone()).unwrap();
        assert!(server.stop());
        wait_until_stopped(&mut server);
        server.start(log_tx).unwrap();
        let _active_connection = (0..50)
            .find_map(
                |_| match std::net::TcpStream::connect(("127.0.0.1", dap_port)) {
                    Ok(stream) => Some(stream),
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(10));
                        None
                    }
                },
            )
            .expect("DAP server did not accept connections");
        assert!(server.stop());
        wait_until_stopped(&mut server);

        TcpListener::bind(("127.0.0.1", dap_port)).unwrap();
        TcpListener::bind(("127.0.0.1", upload_port)).unwrap();
    }
}
