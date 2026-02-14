#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use crate::parameter;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Project {
    pub name: String,
    path: String,
}

impl Project {
    pub fn get_path(&self) -> String {
        std::path::Path::new(&self.path)
            .join(&self.name)
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn is_folder_exists(&self) -> bool {
        std::path::Path::new(&self.get_path()).exists()
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
    child: Option<std::process::Child>,
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
            child: None,
            status: DapServerStatus::Stopped,
        }
    }
}

impl NewProject {
    pub fn history_push(&mut self) -> bool {
        if self.history.contains(&Project {
            name: self.name.clone(),
            path: self.path.clone(),
        }) {
            return false;
        }

        if self.history.len() == self.history_max {
            self.history.remove(0);
        }

        self.history.push(Project {
            name: self.name.clone(),
            path: self.path.clone(),
        });
        true
    }
}

pub fn open_vscode(path: &str) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        let env_path = std::env::var(&"PATH").unwrap();
        std::process::Command::new("code.cmd")
            .arg(path)
            .env("PATH", env_path)
            .output()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg("Visual Studio Code")
            .arg(path) // ディレクトリを指定
            .output()
    }
}

pub fn start_rd() -> std::result::Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let path = std::env!("PATH");
        match std::process::Command::new("rdctl")
            .arg("start")
            .arg("--application.start-in-background")
            .env("PATH", path)
            .creation_flags(0x08000000)
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error: {}, $PATH: {}", e, path)),
        }
    }
    #[cfg(target_os = "macos")]
    {
        let home_dir = std::env::var("HOME").unwrap();
        let zshrc_path = format!("{}/.zshrc", home_dir);

        // .zshrcをsourceしてrdctlを実行
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

pub fn get_probe_rs_versions() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let probe_rs_cmd = "probe-rs";
        let path = std::env!("PATH");
        match std::process::Command::new(probe_rs_cmd)
            .env("PATH", path)
            .arg("--version")
            .output()
        {
            Ok(output) => {
                if let Some(probe_rs_version) = String::from_utf8_lossy(&output.stdout)
                    .to_string()
                    .lines()
                    .next()
                {
                    return Some(probe_rs_version.to_owned());
                };
                return None;
            }
            Err(_) => return None,
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home_dir = std::env::var("HOME").unwrap();
        let zshrc_path = format!("{}/.zshrc", home_dir);

        // .zshrcをsourceしてrdctlを実行
        match std::process::Command::new("zsh")
            .arg("-c")
            .arg(format!("source {} && probe-rs --version", zshrc_path))
            .output()
        {
            Ok(output) => {
                if let Some(probe_rs_version) = String::from_utf8_lossy(&output.stdout)
                    .to_string()
                    .lines()
                    .next()
                {
                    return Some(probe_rs_version.to_owned());
                };
                return None;
            }
            Err(_) => None,
        }
    }
}

pub fn is_docker_running() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let path = std::env!("PATH");
        match Command::new("docker")
            .arg("info")
            .arg("--format")
            .arg("{{.ServerVersion}}")
            .env("PATH", path)
            .output()
        {
            Ok(output) => Ok(output.status.success()),
            Err(e) => Err(format!("docker info failed: {}", e)),
        }
    }
    #[cfg(target_os = "macos")]
    {
        match Command::new("docker")
            .arg("info")
            .arg("--format")
            .arg("{{.ServerVersion}}")
            .output()
        {
            Ok(output) => Ok(output.status.success()),
            Err(e) => Err(format!("docker info failed: {}", e)),
        }
    }
}

impl ProbeRsDapServer {
    pub fn start(&mut self, tx: std::sync::mpsc::Sender<String>) {
        if self.status != DapServerStatus::Stopped {
            return;
        }
        let path = std::env!("PATH");
        #[cfg(target_os = "windows")]
        {
            let mut cmd: Command = Command::new("probe-rs");
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = cmd
                .env("PATH", path)
                .arg("dap-server")
                .arg("--port")
                .arg(self.port.to_string())
                .creation_flags(0x08000000)
                .spawn()
                .unwrap();

            let output = child.stderr.take().unwrap();
            self.child = Some(child);

            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(output);
                for line in std::io::BufRead::lines(reader) {
                    let now = chrono::Local::now();
                    let timestamp = now.format(TIMESTAMP_FORMAT).to_string();
                    let msg = format!("{}: {}", timestamp, line.unwrap());
                    tx.send(msg).unwrap();
                }
            });
        }

        #[cfg(target_os = "macos")]
        {
            let home_dir = std::env::var("HOME").unwrap();
            let zshrc_path = format!("{}/.zshrc", home_dir);

            let mut cmd: Command = Command::new("zsh");
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = cmd
                .arg("-c")
                .arg(format!(
                    "source {} && probe-rs dap-server --port {}",
                    zshrc_path, self.port
                ))
                .env("RUST_LOG", "debug")
                .spawn()
                .unwrap();

            let output = child.stderr.take().unwrap();
            self.child = Some(child);

            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(output);
                for line in std::io::BufRead::lines(reader) {
                    let now = chrono::Local::now();
                    let timestamp = now.format(TIMESTAMP_FORMAT).to_string();
                    let msg = format!("{}: {}", timestamp, line.unwrap());
                    tx.send(msg).unwrap();
                }
            });
        }
        self.status = DapServerStatus::Running(self.port.parse().unwrap());
    }

    pub fn stop(&mut self) -> bool {
        if self.status == DapServerStatus::Stopped {
            return false;
        }
        self.child.as_mut().unwrap().kill().unwrap();
        self.status = DapServerStatus::Stopped;
        true
    }
}

pub fn are_apps_runnning(app_name: &str) -> bool {
    let mut system = sysinfo::System::new_all();
    system.refresh_processes();
    let count = system
        .processes()
        .values()
        .filter(|p| p.name() == app_name)
        .count();
    return count >= 2;
}
