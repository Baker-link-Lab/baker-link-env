use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

const OVERRIDE: &str = "../.docker/compose.host.yaml";

#[derive(Deserialize)]
pub struct DockerInfo {
    #[serde(rename = "OperatingSystem")]
    pub operating_system: String,
    #[serde(rename = "OSType")]
    os_type: String,
    #[serde(rename = "SecurityOptions")]
    security_options: Vec<String>,
}

pub fn needs_host_gateway(info: &DockerInfo, host_os: &str) -> Result<bool> {
    if info.os_type != "linux" {
        bail!("Baker Link Dev Containers require a Linux Docker engine");
    }

    let runtime = info.operating_system.to_ascii_lowercase();
    if runtime.contains("rancher desktop") || runtime.contains("docker desktop") {
        return Ok(false);
    }
    if host_os != "linux" {
        bail!(
            "Unsupported Docker runtime on {host_os}: {}",
            info.operating_system
        );
    }
    if info
        .security_options
        .iter()
        .any(|option| option.contains("rootless"))
    {
        bail!("Rootless Docker requires an explicit host connection setup; automatic host-gateway configuration is not supported");
    }
    Ok(true)
}

pub fn validate_local_endpoint(endpoint: &str) -> Result<()> {
    if !endpoint.trim().starts_with("unix://") {
        bail!("Automatic host-gateway setup requires a local Docker socket; remote Docker endpoints need explicit host connection settings");
    }
    Ok(())
}

fn update_override(config: &mut Value, service: &str, host_gateway: bool) -> Result<()> {
    let root = config
        .as_object_mut()
        .context("Host override must be a JSON object")?;
    let services = root
        .entry("services")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("Host override services must be an object")?;
    let service = services
        .entry(service)
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("Host override service must be an object")?;
    let hosts = service
        .entry("extra_hosts")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("Host override extra_hosts must use object syntax")?;
    if host_gateway {
        hosts.insert("host.docker.internal".into(), json!("host-gateway"));
    } else {
        hosts.remove("host.docker.internal");
    }
    Ok(())
}

// Only migrate the known Baker Link template, not arbitrary user Compose projects.
fn template_files(path: &Path) -> Result<Option<(Value, Vec<String>, String)>> {
    let config_path = path.join(".devcontainer").join("devcontainer.json");
    if !config_path.is_file() {
        return Ok(None);
    }
    let original = std::fs::read_to_string(&config_path)?;
    let config: Value = json5::from_str(&original)?;
    let compose_files = match &config["dockerComposeFile"] {
        Value::String(file) => vec![file.clone()],
        Value::Array(files) => files
            .iter()
            .map(|file| {
                file.as_str()
                    .map(str::to_owned)
                    .context("dockerComposeFile entries must be strings")
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Ok(None),
    };
    if !compose_files
        .iter()
        .any(|file| file == "../.docker/compose.yaml")
    {
        return Ok(None);
    }
    let compose_path = path.join(".docker").join("compose.yaml");
    let compose = std::fs::read_to_string(&compose_path)?;
    if !compose.contains("ghcr.io/baker-link-lab/bakerlink-env:") {
        return Ok(None);
    }
    Ok(Some((config, compose_files, compose)))
}

pub fn is_managed_project(path: &Path) -> Result<bool> {
    Ok(template_files(path)?.is_some())
}

pub fn prepare_project(path: &Path, host_gateway: bool) -> Result<bool> {
    let Some((mut config, compose_files, compose)) = template_files(path)? else {
        return Ok(false);
    };
    let config_path = path.join(".devcontainer").join("devcontainer.json");
    let compose_path = path.join(".docker").join("compose.yaml");
    let normalized = compose.replace("\r\n", "\n");
    let migrated = normalized.replace(
        "    extra_hosts:\n      - \"host.docker.internal:host-gateway\"\n",
        "",
    );
    if migrated
        .lines()
        .any(|line| !line.trim_start().starts_with('#') && line.contains("host.docker.internal"))
    {
        bail!("Custom host.docker.internal mapping in {} must be removed or moved to compose.host.yaml before automatic setup", compose_path.display());
    }
    let service = config["service"]
        .as_str()
        .context("Dev Container service is missing")?;
    let override_path = path.join(".docker").join("compose.host.yaml");
    let mut override_config = match std::fs::read_to_string(&override_path) {
        Ok(contents) => json5::from_str(&contents)
            .with_context(|| format!("Host override must be JSON: {}", override_path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to read {}", override_path.display()))
        }
    };
    update_override(&mut override_config, service, host_gateway)?;
    let override_data = serde_json::to_string_pretty(&override_config)? + "\n";
    write_if_changed(&override_path, &override_data)?;

    if migrated != normalized {
        write_if_changed(&compose_path, &migrated)?;
    }
    if !compose_files.iter().any(|file| file == OVERRIDE) {
        let mut files = compose_files;
        files.push(OVERRIDE.to_string());
        config["dockerComposeFile"] = json!(files);
        write_if_changed(
            &config_path,
            &(serde_json::to_string_pretty(&config)? + "\n"),
        )?;
    }
    Ok(true)
}

fn write_if_changed(path: &Path, contents: &str) -> Result<()> {
    match std::fs::read_to_string(path) {
        Ok(existing) if existing == contents => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", path.display()))
        }
    }
    std::fs::write(path, contents).with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(os: &str, security: Vec<String>) -> DockerInfo {
        DockerInfo {
            operating_system: os.into(),
            os_type: "linux".into(),
            security_options: security,
        }
    }

    #[test]
    fn desktop_runtimes_preserve_native_dns_on_all_hosts() {
        for host in ["windows", "macos", "linux"] {
            for runtime in [
                "Rancher Desktop WSL Distribution (containerized)",
                "Rancher Desktop",
                "Docker Desktop",
            ] {
                assert!(!needs_host_gateway(&info(runtime, vec![]), host).unwrap());
            }
        }
    }

    #[test]
    fn native_debian_uses_host_gateway_but_unsupported_runtimes_fail() {
        assert!(needs_host_gateway(&info("Debian GNU/Linux 12", vec![]), "linux").unwrap());
        assert!(needs_host_gateway(&info("Debian", vec![]), "windows").is_err());
        assert!(
            needs_host_gateway(&info("Debian", vec!["name=rootless".into()]), "linux").is_err()
        );
        let mut windows = info("Windows", vec![]);
        windows.os_type = "windows".into();
        assert!(needs_host_gateway(&windows, "windows").is_err());
    }

    #[test]
    fn desktop_and_linux_overrides_use_the_same_service() {
        let mut config = json!({});
        update_override(&mut config, "myproject1", false).unwrap();
        assert_eq!(
            config,
            json!({"services": {"myproject1": {"extra_hosts": {}}}})
        );
        update_override(&mut config, "myproject1", true).unwrap();
        assert_eq!(
            config,
            json!({"services": {"myproject1": {"extra_hosts": {"host.docker.internal": "host-gateway"}}}})
        );
    }

    #[test]
    fn preserves_other_override_settings_and_rejects_invalid_shapes() {
        let mut config = json!({"services": {"project": {"extra_hosts": {"other.local": "10.0.0.1"}, "environment": {"KEEP": "yes"}}, "other": {"image": "other"}}});
        let original = config.clone();
        update_override(&mut config, "project", true).unwrap();
        update_override(&mut config, "project", false).unwrap();
        assert_eq!(config, original);
        for mut invalid in [
            json!([]),
            json!({"services": []}),
            json!({"services": {"project": {"extra_hosts": []}}}),
        ] {
            assert!(update_override(&mut invalid, "project", true).is_err());
        }
    }

    #[test]
    fn migrates_existing_template_and_switches_hosts_without_duplicate_overrides() {
        let root = std::env::temp_dir().join(format!(
            "bakerlink-network-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join(".docker")).unwrap();
        std::fs::create_dir_all(root.join(".devcontainer")).unwrap();
        assert!(!is_managed_project(&root).unwrap());
        std::fs::write(root.join(".devcontainer").join("devcontainer.json"), r#"{"dockerComposeFile":"../.docker/compose.yaml","service":"project","workspaceFolder":"/project"}"#).unwrap();
        std::fs::write(root.join(".docker").join("compose.yaml"), "services:\n  project:\n    image: ghcr.io/baker-link-lab/bakerlink-env:latest\n    extra_hosts:\n      - \"host.docker.internal:host-gateway\"\n").unwrap();
        assert!(prepare_project(&root, false).unwrap());
        assert!(
            !std::fs::read_to_string(root.join(".docker").join("compose.yaml"))
                .unwrap()
                .contains("host-gateway")
        );
        assert!(prepare_project(&root, true).unwrap());
        assert!(
            std::fs::read_to_string(root.join(".docker").join("compose.host.yaml"))
                .unwrap()
                .contains("host-gateway")
        );
        assert!(prepare_project(&root, false).unwrap());
        let config: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(".devcontainer").join("devcontainer.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            config["dockerComposeFile"],
            json!(["../.docker/compose.yaml", OVERRIDE])
        );
        assert!(
            !std::fs::read_to_string(root.join(".docker").join("compose.host.yaml"))
                .unwrap()
                .contains("host-gateway")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
