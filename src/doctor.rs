use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Passed,
    NeedsAttention,
    NotChecked,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::NeedsAttention => "Needs attention",
            Self::NotChecked => "Not checked",
        }
    }
}

#[derive(Clone)]
pub struct Check {
    pub name: &'static str,
    pub status: Status,
    pub message: String,
    pub details: String,
    pub help_section: Option<&'static str>,
}

#[derive(Clone)]
pub struct Report {
    pub checked_at: String,
    pub checks: Vec<Check>,
}

impl Report {
    pub fn ready(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status == Status::Passed)
    }

    pub fn summary(&self) -> &'static str {
        if self.ready() {
            "Ready to create projects."
        } else {
            "Some checks need your attention. Follow the steps below."
        }
    }

    #[cfg(test)]
    pub fn text(&self) -> String {
        self.localized_text(crate::i18n::Language::English)
    }

    pub fn localized_text(&self, language: crate::i18n::Language) -> String {
        let mut text = format!(
            "{}\n{}\n{}\n{}\n",
            language.text("Baker Link Doctor"),
            language.text(&format!("Host: {}", std::env::consts::OS)),
            language.text(&format!("Checked: {}", self.checked_at)),
            language.text(self.summary())
        );
        for check in &self.checks {
            text.push_str(&format!(
                "\n{}: {}\n{}\n{}\n",
                language.text(check.name),
                language.text(check.status.label()),
                language.text(&check.message),
                language.text(&check.details)
            ));
        }
        text
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Probe {
    VsCode,
    Extensions,
    DockerCli,
    DockerEngine,
    DockerEndpoint,
}

const VSCODE_HELP: &str = "doctor-vscode";
const EXTENSION_HELP: &str = "doctor-dev-containers";
const DOCKER_HELP: &str = "doctor-docker-cli";

pub fn run() -> Report {
    diagnose(std::env::consts::OS, |probe| {
        let mut command = match probe {
            Probe::VsCode | Probe::Extensions => crate::cmd::vscode_command(),
            _ => crate::cmd::docker_command(),
        };
        match probe {
            Probe::VsCode => command.args(["--version"]),
            Probe::Extensions => command.args(["--list-extensions"]),
            Probe::DockerCli => command.args(["--version"]),
            Probe::DockerEngine => command.args(["info", "--format", "{{json .}}"]),
            Probe::DockerEndpoint => {
                if std::env::var_os("DOCKER_CONTEXT").is_none() {
                    if let Ok(host) = std::env::var("DOCKER_HOST") {
                        if !host.is_empty() {
                            return Ok(host);
                        }
                    }
                }
                command.args([
                    "context",
                    "inspect",
                    "--format",
                    "{{.Endpoints.docker.Host}}",
                ])
            }
        };
        let text = command_text(command, Duration::from_secs(15))?;
        if text.is_empty() && probe != Probe::Extensions {
            return Err("Command returned an empty result.".into());
        }
        Ok(text)
    })
}

fn diagnose(host_os: &str, mut probe: impl FnMut(Probe) -> Result<String, String>) -> Report {
    let mut checks = Vec::new();
    let mut add = |name, status, message: &str, details: String, help_section| {
        let help_section = if status == Status::Passed {
            None
        } else {
            match name {
                "Docker Engine" => Some("doctor-docker-engine"),
                "Host networking support" => Some("doctor-host-networking"),
                _ => help_section,
            }
        };
        checks.push(Check {
            name,
            status,
            message: message.into(),
            details,
            help_section,
        });
    };
    match probe(Probe::VsCode) {
        Ok(version) => {
            add(
                "VS Code",
                Status::Passed,
                "VS Code CLI is available.",
                version,
                None,
            );
            match probe(Probe::Extensions) {
                Ok(extensions) if crate::cmd::has_dev_containers(&extensions) => {
                    add(
                        "Dev Containers",
                        Status::Passed,
                        "Dev Containers is installed in the host VS Code.",
                        String::new(),
                        None,
                    );
                }
                Ok(_) => add(
                    "Dev Containers",
                    Status::NeedsAttention,
                    "Install the Dev Containers extension in the host VS Code.",
                    String::new(),
                    Some(EXTENSION_HELP),
                ),
                Err(error) => add(
                    "Dev Containers",
                    Status::NeedsAttention,
                    "Could not list host VS Code extensions. Check the CLI error below.",
                    error,
                    Some(EXTENSION_HELP),
                ),
            }
        }
        Err(error) => {
            add(
                "VS Code",
                Status::NeedsAttention,
                "Could not run VS Code. Install it or make its code CLI available.",
                error,
                Some(VSCODE_HELP),
            );
            add(
                "Dev Containers",
                Status::NotChecked,
                "Check VS Code first, then run Doctor again.",
                String::new(),
                Some(EXTENSION_HELP),
            );
        }
    }
    match probe(Probe::DockerCli) {
        Err(error) => {
            add(
                "Docker CLI",
                Status::NeedsAttention,
                "Could not run Docker. Install a Docker runtime and check PATH.",
                error,
                Some(DOCKER_HELP),
            );
            add(
                "Docker Engine",
                Status::NotChecked,
                "Docker CLI is required to check the engine.",
                String::new(),
                None,
            );
            add(
                "Host networking support",
                Status::NotChecked,
                "Docker Engine information is required.",
                String::new(),
                None,
            );
        }
        Ok(version) => {
            add(
                "Docker CLI",
                Status::Passed,
                "Docker CLI is available.",
                version,
                None,
            );
            match probe(Probe::DockerEngine) {
                Err(error) => {
                    add("Docker Engine", Status::NeedsAttention, "Cannot reach Docker Engine. Start your runtime and check the active Docker context.", error, Some(DOCKER_HELP));
                    add(
                        "Host networking support",
                        Status::NotChecked,
                        "Start Docker Engine, then run Doctor again.",
                        String::new(),
                        None,
                    );
                }
                Ok(details) => {
                    match serde_json::from_str::<crate::host_network::DockerInfo>(&details) {
                        Err(error) => {
                            add(
                                "Docker Engine",
                                Status::NeedsAttention,
                                "Docker returned invalid runtime information.",
                                error.to_string(),
                                None,
                            );
                            add(
                                "Host networking support",
                                Status::NotChecked,
                                "Valid Docker Engine information is required.",
                                String::new(),
                                None,
                            );
                        }
                        Ok(info) => {
                            add(
                                "Docker Engine",
                                Status::Passed,
                                "Docker Engine is reachable.",
                                format!("Runtime: {}", info.operating_system),
                                None,
                            );
                            let network = crate::host_network::needs_host_gateway(&info, host_os)
                                .map_err(|error| error.to_string())
                                .and_then(|gateway| {
                                    if gateway {
                                        let endpoint = probe(Probe::DockerEndpoint)?;
                                        crate::host_network::validate_local_endpoint(&endpoint)
                                            .map_err(|error| error.to_string())?;
                                    }
                                    Ok(gateway)
                                });
                            match network {
                                Ok(gateway) => add("Host networking support", Status::Passed, if gateway { "Local Docker Engine supports the Linux host-gateway setup." } else { "Desktop runtime provides native host DNS." }, "Runtime compatibility only; no container or host port connection was tested.".into(), None),
                                Err(error) => add("Host networking support", Status::NeedsAttention, "This runtime needs a different host connection setup.", error, Some(DOCKER_HELP)),
                            }
                        }
                    }
                }
            }
        }
    }
    Report {
        checked_at: chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %:z")
            .to_string(),
        checks,
    }
}

fn command_text(mut command: Command, timeout: Duration) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to execute command: {error}"))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let read = |mut stream: Box<dyn Read + Send>| {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = tx.send(stream.read_to_end(&mut bytes).map(|_| bytes));
        });
        rx
    };
    let out = read(Box::new(stdout));
    let err = read(Box::new(stderr));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            result => {
                let reason = match result {
                    Err(error) => format!("Failed to inspect command: {error}"),
                    _ => format!("Command timed out after {} seconds.", timeout.as_secs()),
                };
                // Descendant processes may retain the pipe handles; do not wait on readers here.
                let cleanup = child.kill().and_then(|_| child.wait().map(|_| ()));
                return Err(match cleanup {
                    Ok(()) => reason,
                    Err(error) => format!("{reason} Failed to stop command: {error}"),
                });
            }
        }
    };
    let stdout = out
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| format!("Command stdout timed out or reader failed: {error}"))?
        .map_err(|error| format!("Failed to read command stdout: {error}"))?;
    let stderr = err
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| format!("Command stderr timed out or reader failed: {error}"))?
        .map_err(|error| format!("Failed to read command stderr: {error}"))?;
    let stdout = String::from_utf8_lossy(&stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
    if !status.success() {
        return Err(format!("Command failed ({status}).\n{stdout}\n{stderr}")
            .trim()
            .to_string());
    }
    if stdout.is_empty() && !stderr.is_empty() {
        return Err(format!("Command returned no result: {stderr}"));
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy(probe: Probe) -> Result<String, String> {
        Ok(match probe {
            Probe::VsCode => "1.116.0".into(),
            Probe::Extensions => {
                "MS-VSCODE-REMOTE.REMOTE-CONTAINERS\nrust-lang.rust-analyzer".into()
            }
            Probe::DockerCli => "Docker version 28".into(),
            Probe::DockerEngine => {
                r#"{"OperatingSystem":"Rancher Desktop","OSType":"linux","SecurityOptions":[]}"#
                    .into()
            }
            Probe::DockerEndpoint => "unix:///var/run/docker.sock".into(),
        })
    }

    #[test]
    fn healthy_desktop_is_ready_on_all_hosts() {
        for host in ["windows", "macos", "linux"] {
            let report = diagnose(host, healthy);
            assert!(report.ready());
            assert_eq!(report.checks.len(), 5);
            assert!(report.text().contains("Runtime: Rancher Desktop"));
            assert!(report
                .text()
                .contains("no container or host port connection was tested"));
        }
    }

    #[test]
    #[ignore = "Runs installed host tools; execute manually for environment smoke testing"]
    fn installed_host_tools_return_a_complete_report() {
        let report = run();
        assert_eq!(report.checks.len(), 5);
        assert!(!report.checked_at.is_empty());
        for check in &report.checks {
            assert!(!check.message.is_empty());
        }
        println!("{}", report.text());
    }

    #[test]
    fn missing_tools_skip_dependents_without_running_them() {
        let report = diagnose("windows", |probe| {
            assert!(matches!(probe, Probe::VsCode | Probe::DockerCli));
            Err("not found".into())
        });
        assert!(!report.ready());
        assert_eq!(
            report
                .checks
                .iter()
                .filter(|check| check.status == Status::NotChecked)
                .count(),
            3
        );
    }

    #[test]
    fn stopped_engine_and_missing_extension_show_actions() {
        let report = diagnose("macos", |probe| match probe {
            Probe::Extensions => Ok(String::new()),
            Probe::DockerEngine => Err("connection refused".into()),
            _ => healthy(probe),
        });
        assert!(!report.ready());
        assert_eq!(report.checks[1].help_section, Some(EXTENSION_HELP));
        assert_eq!(report.checks[4].status, Status::NotChecked);
        assert!(report.text().contains("connection refused"));
    }

    #[test]
    fn linux_checks_local_endpoint_and_rejects_remote_and_rootless() {
        for (security, endpoint, ready) in [
            ("[]", "unix:///var/run/docker.sock", true),
            ("[]", "ssh://remote", false),
            (
                r#"["name=rootless"]"#,
                "unix:///run/user/1000/docker.sock",
                false,
            ),
        ] {
            let report = diagnose("linux", |probe| match probe {
                Probe::DockerEngine => Ok(format!(
                    r#"{{"OperatingSystem":"Debian","OSType":"linux","SecurityOptions":{security}}}"#
                )),
                Probe::DockerEndpoint => Ok(endpoint.into()),
                _ => healthy(probe),
            });
            assert_eq!(report.ready(), ready);
        }
    }

    #[test]
    fn malformed_engine_information_is_not_success() {
        let report = diagnose("windows", |probe| {
            if probe == Probe::DockerEngine {
                Ok("invalid".into())
            } else {
                healthy(probe)
            }
        });
        assert_eq!(report.checks[3].status, Status::NeedsAttention);
        assert_eq!(report.checks[4].status, Status::NotChecked);
    }

    #[test]
    fn help_links_cover_every_failed_check_in_both_languages() {
        let report = diagnose("windows", |_| Err("not found".into()));
        for check in &report.checks {
            let section = check
                .help_section
                .expect("Every failed or skipped check needs help");
            for (language, readme) in [
                (crate::i18n::Language::English, include_str!("../README.md")),
                (
                    crate::i18n::Language::Japanese,
                    include_str!("../README.ja.md"),
                ),
            ] {
                assert!(readme.contains(&format!("## {section}")));
                assert!(language
                    .help_url(Some(section))
                    .ends_with(&format!("#{section}")));
            }
        }
        assert!(crate::i18n::Language::English
            .help_url(None)
            .ends_with("/README.md"));
        assert!(crate::i18n::Language::Japanese
            .help_url(None)
            .ends_with("/README.ja.md"));
        assert!(diagnose("windows", healthy)
            .checks
            .iter()
            .all(|check| check.help_section.is_none()));
    }

    #[test]
    fn existing_reports_translate_without_changing_results() {
        use crate::i18n::Language;
        for report in [
            diagnose("windows", healthy),
            diagnose("windows", |_| Err("command not found".into())),
            diagnose("linux", |probe| match probe {
                Probe::DockerEngine => Err("ECONNREFUSED 127.0.0.1".into()),
                Probe::Extensions => Ok(String::new()),
                _ => healthy(probe),
            }),
        ] {
            let ready = report.ready();
            for check in &report.checks {
                assert_ne!(Language::Japanese.text(&check.message), check.message);
                assert_ne!(
                    Language::Japanese.text(check.status.label()),
                    check.status.label()
                );
            }
            let japanese = report.localized_text(Language::Japanese);
            assert!(japanese.contains("診断日時: "));
            assert!(!japanese.contains("Needs attention"));
            let english = report.localized_text(Language::English);
            assert_eq!(english, report.text());
            assert_eq!(report.ready(), ready);
        }
    }

    #[test]
    fn command_runner_reports_success_failure_and_timeout() {
        #[cfg(target_os = "windows")]
        fn shell(script: &str) -> Command {
            use std::os::windows::process::CommandExt;
            let mut command = Command::new("cmd.exe");
            command
                .args(["/D", "/C", script])
                .creation_flags(0x08000000);
            command
        }
        #[cfg(not(target_os = "windows"))]
        fn shell(script: &str) -> Command {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            command
        }
        assert_eq!(
            command_text(shell("echo doctor-test"), Duration::from_secs(5)).unwrap(),
            "doctor-test"
        );
        assert!(command_text(shell("exit 7"), Duration::from_secs(5))
            .unwrap_err()
            .contains("Command failed"));
        let started = Instant::now();
        let script = if cfg!(target_os = "windows") {
            "for /L %i in (0,0,1) do @rem"
        } else {
            "while :; do :; done"
        };
        let error = command_text(shell(script), Duration::from_millis(50)).unwrap_err();
        assert!(error.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
