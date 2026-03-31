/// DAP (Debug Adapter Protocol) TCP proxy with path translation.
///
/// Sits between a Docker container client and the probe-rs DAP server,
/// translating container-side Linux paths to host-side paths and back.
///
/// Architecture:
///   Docker/VS Code client  -->  [DapProxy :external_port]  -->  [probe-rs :internal_port]
///
/// Direction rules:
///   Client -> Server (incoming): container path  -->  host path
///   Server -> Client (outgoing): host path       -->  container path
use crate::cmd::PathMapping;
use serde_json::Value;
use std::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

// ─── Path Translator ────────────────────────────────────────────────────────

/// Translates file paths between container (Linux) format and host (Win/macOS) format.
#[derive(Clone, Debug)]
pub struct PathTranslator {
    mappings: Vec<PathMapping>,
}

impl PathTranslator {
    pub fn new(mappings: Vec<PathMapping>) -> Self {
        Self { mappings }
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    /// Container path → host path.
    /// e.g. `/workspaces/myproject/src/main.rs` → `C:\Users\...\src\main.rs`
    pub fn container_to_host(&self, path: &str) -> String {
        for m in &self.mappings {
            if path.starts_with(&m.container_path) {
                let suffix = &path[m.container_path.len()..];
                // On Windows the host uses backslashes
                #[cfg(target_os = "windows")]
                let host_suffix = suffix.replace('/', "\\");
                #[cfg(not(target_os = "windows"))]
                let host_suffix = suffix.to_string();
                return format!("{}{}", m.host_path, host_suffix);
            }
        }
        path.to_string()
    }

    /// Host path → container path.
    /// e.g. `C:\Users\...\src\main.rs` → `/workspaces/myproject/src/main.rs`
    pub fn host_to_container(&self, path: &str) -> String {
        for m in &self.mappings {
            // Normalize both sides to forward slashes for comparison
            let norm_host = m.host_path.replace('\\', "/");
            let norm_input = path.replace('\\', "/");
            if norm_input.starts_with(&norm_host) {
                let suffix = &norm_input[norm_host.len()..];
                return format!("{}{}", m.container_path, suffix);
            }
        }
        path.to_string()
    }

    /// Recursively walk JSON and translate all string values.
    fn translate_json(&self, value: &mut Value, container_to_host: bool) {
        match value {
            Value::String(s) => {
                let translated = if container_to_host {
                    self.container_to_host(s)
                } else {
                    self.host_to_container(s)
                };
                if translated != *s {
                    *s = translated;
                }
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    self.translate_json(v, container_to_host);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.translate_json(v, container_to_host);
                }
            }
            _ => {}
        }
    }
}

// ─── DAP Framing ────────────────────────────────────────────────────────────

/// Read one DAP message (Content-Length header + JSON body) from an async reader.
/// Returns the raw JSON body bytes.
async fn read_dap_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    let mut content_length: Option<usize> = None;

    // Read headers byte-by-byte until empty line
    loop {
        let mut line_buf = Vec::new();
        loop {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b).await?;
            if b[0] == b'\n' {
                if line_buf.last() == Some(&b'\r') {
                    line_buf.pop();
                }
                break;
            }
            line_buf.push(b[0]);
        }
        if line_buf.is_empty() {
            break; // blank line = end of headers
        }
        let line = String::from_utf8_lossy(&line_buf);
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            if let Ok(n) = rest.trim().parse::<usize>() {
                content_length = Some(n);
            }
        }
    }

    let length = content_length.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "DAP message missing Content-Length header",
        )
    })?;

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).await?;
    Ok(body)
}

/// Wrap a JSON body in a DAP Content-Length frame.
fn encode_dap_message(body: &[u8]) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = header.into_bytes();
    out.extend_from_slice(body);
    out
}

// ─── Translation helpers ─────────────────────────────────────────────────────

/// Parse JSON body, apply path translation, re-serialize.
/// `container_to_host`: true for client→server direction, false for server→client.
/// Returns the (possibly modified) body, ready to be re-framed.
fn translate_body(
    body: &[u8],
    translator: &PathTranslator,
    container_to_host: bool,
    log_tx: &mpsc::Sender<String>,
) -> Vec<u8> {
    let mut json: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            let _ = log_tx.send(format!(
                "[DAP Proxy] JSON parse error ({}): {}",
                if container_to_host { "c→h" } else { "h→c" },
                e
            ));
            return body.to_vec();
        }
    };

    translator.translate_json(&mut json, container_to_host);

    match serde_json::to_vec(&json) {
        Ok(out) => {
            if out != body {
                let dir = if container_to_host { "c→h" } else { "h→c" };
                let _ = log_tx.send(format!("[DAP Proxy] Path translated ({})", dir));
                let _ = log_tx.send(format!(
                    "[DAP Proxy] before: {}",
                    String::from_utf8_lossy(body)
                ));
                let _ = log_tx.send(format!(
                    "[DAP Proxy]  after: {}",
                    String::from_utf8_lossy(&out)
                ));
            }
            out
        }
        Err(e) => {
            let _ = log_tx.send(format!("[DAP Proxy] JSON serialize error: {}", e));
            body.to_vec()
        }
    }
}

// ─── Connection handler ──────────────────────────────────────────────────────

/// Proxy one client connection to the internal DAP server.
async fn handle_connection(
    client_stream: TcpStream,
    internal_port: u16,
    translator: PathTranslator,
    log_tx: mpsc::Sender<String>,
) {
    // Connect to the internal probe-rs DAP server (with retries, it may not be up yet)
    let server_stream = {
        let mut stream = None;
        for attempt in 1..=15 {
            match TcpStream::connect(format!("127.0.0.1:{}", internal_port)).await {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => {
                    if attempt == 15 {
                        let _ = log_tx.send(format!(
                            "[DAP Proxy] Cannot connect to internal port {} after 15 attempts: {}",
                            internal_port, e
                        ));
                        return;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
        }
        stream.unwrap()
    };

    let _ = log_tx.send(format!(
        "[DAP Proxy] Connected to probe-rs on 127.0.0.1:{}",
        internal_port
    ));

    let (mut cr, mut cw) = client_stream.into_split();
    let (mut sr, mut sw) = server_stream.into_split();

    let log_c2s = log_tx.clone();
    let log_s2c = log_tx.clone();
    let tr_c2s = translator.clone();
    let tr_s2c = translator;

    // Client → Server: translate container paths → host paths
    let c2s = async move {
        loop {
            match read_dap_message(&mut cr).await {
                Ok(body) => {
                    let translated = translate_body(&body, &tr_c2s, true, &log_c2s);
                    let frame = encode_dap_message(&translated);
                    if let Err(e) = sw.write_all(&frame).await {
                        let _ = log_c2s.send(format!("[DAP Proxy] c→s write error: {}", e));
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::UnexpectedEof
                        && e.kind() != std::io::ErrorKind::ConnectionReset
                    {
                        let _ = log_c2s.send(format!("[DAP Proxy] c→s read error: {}", e));
                    }
                    break;
                }
            }
        }
    };

    // Server → Client: translate host paths → container paths
    let s2c = async move {
        loop {
            match read_dap_message(&mut sr).await {
                Ok(body) => {
                    let translated = translate_body(&body, &tr_s2c, false, &log_s2c);
                    let frame = encode_dap_message(&translated);
                    if let Err(e) = cw.write_all(&frame).await {
                        let _ = log_s2c.send(format!("[DAP Proxy] s→c write error: {}", e));
                        break;
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::UnexpectedEof
                        && e.kind() != std::io::ErrorKind::ConnectionReset
                    {
                        let _ = log_s2c.send(format!("[DAP Proxy] s→c read error: {}", e));
                    }
                    break;
                }
            }
        }
    };

    // Run both directions; stop when either side closes
    tokio::select! {
        _ = c2s => {}
        _ = s2c => {}
    }

    let _ = log_tx.send("[DAP Proxy] Connection closed".to_string());
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Run the DAP proxy accept loop.
///
/// Listens on `0.0.0.0:external_port` (accessible from Docker containers) and
/// forwards each connection to `127.0.0.1:internal_port` (probe-rs DAP server),
/// translating paths in both directions.
pub async fn run_dap_proxy(
    external_port: u16,
    internal_port: u16,
    translator: PathTranslator,
    shutdown: CancellationToken,
    log_tx: mpsc::Sender<String>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", external_port)).await?;

    let _ = log_tx.send(format!(
        "[DAP Proxy] Listening on 0.0.0.0:{} \
         (Docker clients should use host.docker.internal:{}) \
         → internal probe-rs on 127.0.0.1:{}",
        external_port, external_port, internal_port
    ));

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                let _ = log_tx.send("[DAP Proxy] Shutting down".to_string());
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        let _ = log_tx.send(format!(
                            "[DAP Proxy] Client connected from {}",
                            addr
                        ));
                        let t = translator.clone();
                        let l = log_tx.clone();
                        tokio::spawn(handle_connection(stream, internal_port, t, l));
                    }
                    Err(e) => {
                        let _ = log_tx.send(format!("[DAP Proxy] Accept error: {}", e));
                    }
                }
            }
        }
    }

    Ok(())
}
