use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tiny_http::{Method, Request, Response, Server, StatusCode};
use tokio_util::sync::CancellationToken;

use crate::artifact;

pub const DEFAULT_UPLOAD_PORT: u16 = 50002;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionConfig<'a> {
    version: u8,
    upload_url: &'a str,
    dap_server: &'a str,
    token: &'a str,
}

pub fn write_connection_config(
    project_path: &Path,
    token: &str,
    connection_host: &str,
    dap_port: u16,
    upload_port: u16,
) -> Result<()> {
    let directory = project_path.join(".bakerlink");
    fs::create_dir_all(&directory)?;
    let upload_url = format!("http://{connection_host}:{upload_port}");
    let dap_server = format!("{connection_host}:{dap_port}");
    let config = ConnectionConfig {
        version: 1,
        upload_url: &upload_url,
        dap_server: &dap_server,
        token,
    };
    let data = serde_json::to_vec_pretty(&config)?;
    let path = directory.join("connection.json");
    fs::write(&path, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    write_vscode_server(project_path, &dap_server)?;
    migrate_launch_server(project_path)?;
    Ok(())
}

fn write_vscode_server(project_path: &Path, dap_server: &str) -> Result<()> {
    let path = project_path.join(".vscode/settings.json");
    let mut settings = match fs::read_to_string(&path) {
        Ok(data) => json5::from_str::<serde_json::Value>(&data)
            .with_context(|| format!("Failed to parse {}", path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(error) => return Err(error.into()),
    };
    let object = settings
        .as_object_mut()
        .context(".vscode/settings.json must contain a JSON object")?;
    object.insert(
        "bakerLink.dapServer".to_string(),
        serde_json::Value::String(dap_server.to_string()),
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&settings)?)?;
    Ok(())
}

fn migrate_launch_server(project_path: &Path) -> Result<()> {
    let path = project_path.join(".vscode/launch.json");
    let Ok(data) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let mut launch: serde_json::Value =
        json5::from_str(&data).with_context(|| format!("Failed to parse {}", path.display()))?;
    let configurations = launch
        .get_mut("configurations")
        .and_then(serde_json::Value::as_array_mut)
        .context("launch.json configurations must be an array")?;
    let mut changed = false;
    for configuration in configurations {
        let Some(object) = configuration.as_object_mut() else {
            continue;
        };
        if object.get("type").and_then(serde_json::Value::as_str) != Some("probe-rs-debug") {
            continue;
        }
        object.insert(
            "server".to_string(),
            serde_json::Value::String("${config:bakerLink.dapServer}".to_string()),
        );
        object.insert(
            "remoteServerMode".to_string(),
            serde_json::Value::Bool(false),
        );
        changed = true;
    }
    if changed {
        fs::write(path, serde_json::to_vec_pretty(&launch)?)?;
    }
    Ok(())
}

pub fn run(
    address: std::net::SocketAddr,
    root: std::path::PathBuf,
    token: String,
    shutdown: CancellationToken,
    log_tx: std::sync::mpsc::Sender<String>,
) -> Result<()> {
    let server = Server::http(address)
        .map_err(|error| anyhow!("Failed to bind ELF upload server on {address}: {error}"))?;
    let _ = log_tx.send(format!("ELF upload server listening on {address}"));

    while !shutdown.is_cancelled() {
        if let Some(request) = server.recv_timeout(Duration::from_millis(100))? {
            handle_request(request, &root, &token, &log_tx);
        }
    }
    Ok(())
}

fn handle_request(
    mut request: Request,
    root: &Path,
    token: &str,
    log_tx: &std::sync::mpsc::Sender<String>,
) {
    let result = process_request(&mut request, root, token);
    let (status, message) = match result {
        Ok(path) => {
            let message = format!("ELF uploaded: {}", path.display());
            let _ = log_tx.send(message.clone());
            (StatusCode(201), message)
        }
        Err(error) => {
            let message = error.to_string();
            let _ = log_tx.send(format!("[ERROR] ELF upload rejected: {message}"));
            (StatusCode(400), message)
        }
    };
    let _ = request.respond(Response::from_string(message).with_status_code(status));
}

fn process_request(request: &mut Request, root: &Path, token: &str) -> Result<std::path::PathBuf> {
    if request.method() != &Method::Put {
        return Err(anyhow!("Only PUT is supported"));
    }
    let authorization = header_value(request, "Authorization").unwrap_or_default();
    if authorization != format!("Bearer {token}") {
        return Err(anyhow!("Invalid upload token"));
    }
    let checksum =
        header_value(request, "X-Content-SHA256").context("Missing X-Content-SHA256 header")?;
    let content_length = request
        .body_length()
        .context("Missing Content-Length header")? as u64;
    let request_url = request.url().to_string();
    let path = request_url.trim_start_matches('/');
    let mut parts = path.split('/');
    if parts.next() != Some("v1") || parts.next() != Some("artifacts") {
        return Err(anyhow!("Invalid upload URL"));
    }
    let project = parts.next().context("Missing project identifier")?;
    let profile = parts.next().context("Missing profile identifier")?;
    if parts.next().is_some() {
        return Err(anyhow!("Invalid upload URL"));
    }
    artifact::store(
        root,
        project,
        profile,
        &checksum,
        content_length,
        request.as_reader(),
    )
}

fn header_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.to_string().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn uploads_authenticated_artifact() {
        let root = std::env::temp_dir().join(format!(
            "baker-link-upload-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let token = "test-token".to_string();
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        let (log_tx, _log_rx) = std::sync::mpsc::channel();
        let server_root = root.clone();
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let handle = std::thread::spawn(move || {
            run(address, server_root, token, server_shutdown, log_tx).unwrap();
        });

        let mut stream = (0..50)
            .find_map(|_| match TcpStream::connect(address) {
                Ok(stream) => Some(stream),
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(10));
                    None
                }
            })
            .expect("upload server did not start");
        let body = b"uploaded-elf";
        let checksum = format!("{:x}", Sha256::digest(body));
        write!(
            stream,
            "PUT /v1/artifacts/example/debug HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-token\r\nX-Content-SHA256: {checksum}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.starts_with("HTTP/1.1 201"), "{response}");
        assert_eq!(fs::read(root.join("example/debug.elf")).unwrap(), body);
        shutdown.cancel();
        handle.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_existing_vscode_jsonc_settings() {
        let root = std::env::temp_dir().join(format!(
            "baker-link-settings-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".vscode")).unwrap();
        fs::write(
            root.join(".vscode/settings.json"),
            "{\n  // Keep project settings.\n  \"editor.formatOnSave\": true,\n}\n",
        )
        .unwrap();
        fs::write(
            root.join(".vscode/launch.json"),
            "{\n  \"configurations\": [{\n    \"type\": \"probe-rs-debug\",\n    \"server\": \"old-host:50000\",\n    \"remoteServerMode\": true,\n    \"chip\": \"RP2040\"\n  }, {\n    \"type\": \"cppdbg\",\n    \"server\": \"keep-host:1234\",\n    \"remoteServerMode\": true\n  }]\n}\n",
        )
        .unwrap();

        write_connection_config(&root, "token", "host.docker.internal", 50001, 50002).unwrap();

        let settings: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(".vscode/settings.json")).unwrap()).unwrap();
        assert_eq!(settings["editor.formatOnSave"], true);
        assert_eq!(
            settings["bakerLink.dapServer"],
            "host.docker.internal:50001"
        );
        let launch: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(".vscode/launch.json")).unwrap()).unwrap();
        assert_eq!(launch["configurations"][0]["remoteServerMode"], false);
        assert_eq!(launch["configurations"][0]["chip"], "RP2040");
        assert_eq!(launch["configurations"][1]["server"], "keep-host:1234");
        assert_eq!(launch["configurations"][1]["remoteServerMode"], true);
        fs::remove_dir_all(root).unwrap();
    }
}
