use std::process::{Command, Stdio};

use chrono::format;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct NewProject {
    pub name: String,
    pub path: String,
    pub vscode_open_enabled: bool,
    history: Vec<String>,
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
            path: "myproject/path".to_string(),
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
    pub fn history_push(&mut self, path: String) -> bool {
        if self.history.contains(&path) {
            return false;
        }

        if self.history.len() == self.history_max {
            self.history.remove(0);
        }
        self.history.push(path);
        true
    }
}

pub fn open_vscode(path: &str) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    let vscode_cmd = "code.cmd";
    #[cfg(target_os = "macos")]
    let vscode_cmd = "code";

    let env_path = std::env::var(&"PATH").unwrap();
    std::process::Command::new(vscode_cmd)
        .arg(path)
        .env("PATH", env_path)
        .output()
}

pub fn generate_project(name: &str, path: &str) -> anyhow::Result<std::path::PathBuf> {
    std::env::set_current_dir(path).unwrap();
    let generate_args = cargo_generate::GenerateArgs {
        name: Some(name.to_string()),
        vcs: Some(cargo_generate::Vcs::Git),
        template_path: cargo_generate::TemplatePath {
            git: Some("https://github.com/T-ikko/bakerlink_tutorial_template.git".to_string()),
            ..cargo_generate::TemplatePath::default()
        },
        ..cargo_generate::GenerateArgs::default()
    };
    cargo_generate::generate(generate_args)
}

pub fn get_probe_rs_versions() -> Option<String> {
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

impl ProbeRsDapServer {
    pub fn start(&mut self, tx: std::sync::mpsc::Sender<String>) {
        if self.status != DapServerStatus::Stopped {
            return;
        }
        let mut cmd = Command::new("probe-rs");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let path = std::env!("PATH");
        let mut child = cmd
            .env("PATH", path)
            .arg("dap-server")
            .arg("--port")
            .arg(self.port.to_string())
            .spawn()
            .unwrap();

        let output = child.stderr.take().unwrap();
        self.child = Some(child);

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(output);
            for line in std::io::BufRead::lines(reader) {
                let now = chrono::Local::now();
                now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                let msg = format!("{}: {}", now, line.unwrap());
                tx.send(msg).unwrap();
            }
        });
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
