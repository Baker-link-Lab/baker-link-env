use anyhow::{anyhow, Context, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;
use tiny_http::{Method, Request, Response, Server, StatusCode};
use tokio_util::sync::CancellationToken;

use crate::artifact;

pub const DEFAULT_UPLOAD_PORT: u16 = 50002;

pub fn run(
    address: std::net::SocketAddr,
    root: std::path::PathBuf,
    shutdown: CancellationToken,
    log_tx: std::sync::mpsc::Sender<String>,
) -> Result<()> {
    let server = Server::http(address)
        .map_err(|error| anyhow!("Failed to bind ELF upload server on {address}: {error}"))?;
    let _ = log_tx.send(format!("ELF upload server listening on {address}"));

    let result = (|| {
        while !shutdown.is_cancelled() {
            if let Some(request) = server.recv_timeout(Duration::from_millis(100))? {
                handle_request(request, &root, &log_tx);
            }
        }
        Ok(())
    })();

    let listener_address = server
        .server_addr()
        .to_ip()
        .context("ELF upload server has no TCP address")?;
    drop(server);

    // tiny_http wakes accept() by connecting to its bind address, which fails
    // for wildcard addresses on Windows. Wake it through loopback instead.
    if listener_address.ip().is_unspecified() {
        let mut wake_address = listener_address;
        wake_address.set_ip(match wake_address.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
        match TcpStream::connect_timeout(&wake_address, Duration::from_secs(1)) {
            Ok(stream) => drop(stream),
            Err(error) => {
                // Server::drop may have already woken the accept thread. OSes
                // report different connect errors while the listener closes.
                let deadline = std::time::Instant::now() + Duration::from_secs(1);
                loop {
                    match TcpListener::bind(listener_address) {
                        Ok(listener) => {
                            drop(listener);
                            break;
                        }
                        Err(bind_error) => {
                            if std::time::Instant::now() >= deadline {
                                return Err(error).with_context(|| {
                                    format!(
                                        "Failed to wake ELF upload listener; \
                                         {listener_address} is still unavailable: {bind_error}"
                                    )
                                });
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        }
    }
    result
}

fn handle_request(mut request: Request, root: &Path, log_tx: &std::sync::mpsc::Sender<String>) {
    let result = process_request(&mut request, root);
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

fn process_request(request: &mut Request, root: &Path) -> Result<std::path::PathBuf> {
    if request.method() != &Method::Put {
        return Err(anyhow!("Only PUT is supported"));
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
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn immediate_shutdown_releases_listener_before_restart() {
        for ip in [Ipv4Addr::UNSPECIFIED, Ipv4Addr::LOCALHOST] {
            let listener = TcpListener::bind((ip, 0)).unwrap();
            let address = listener.local_addr().unwrap();
            drop(listener);
            let (log_tx, _log_rx) = std::sync::mpsc::channel();

            for _ in 0..20 {
                let shutdown = CancellationToken::new();
                shutdown.cancel();
                run(address, std::env::temp_dir(), shutdown, log_tx.clone()).unwrap();

                let deadline = std::time::Instant::now() + Duration::from_secs(3);
                loop {
                    match TcpListener::bind(address) {
                        Ok(listener) => {
                            drop(listener);
                            break;
                        }
                        Err(error) => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "upload port was not released: {error}"
                            );
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn uploads_artifact_without_authentication() {
        let root = std::env::temp_dir().join(format!(
            "baker-link-upload-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();
        let (log_tx, _log_rx) = std::sync::mpsc::channel();
        let server_root = root.clone();
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let address = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let handle = std::thread::spawn(move || {
            run(address, server_root, server_shutdown, log_tx).unwrap();
        });

        let mut stream = (0..50)
            .find_map(|_| match TcpStream::connect(("127.0.0.1", port)) {
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
            "PUT /v1/artifacts/example/debug HTTP/1.1\r\nHost: localhost\r\nX-Content-SHA256: {checksum}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
}
