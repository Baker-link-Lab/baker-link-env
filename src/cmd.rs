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
    pub adderess: String,
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
            adderess: "120.0.0.1".to_string(),
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
