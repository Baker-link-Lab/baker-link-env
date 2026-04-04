---
name: dap-integration
description: "probe-rs DAPサーバー統合。Use when: DAPサーバー修正、デバッグ機能追加、probe-rs連携、ブレークポイント、パスマッピング、CancellationToken、シャットダウン"
---

# probe-rs DAP サーバー統合

## When to Use
- DAP サーバーの起動・停止ロジックを変更するとき
- probe-rs との連携機能を追加するとき
- デバッグ関連（ブレークポイント、パスマッピング等）の修正
- DAP サーバーのライフサイクル管理の変更

## Architecture

```
┌─────────────────────────────┐
│  VS Code (Dev Container内)  │
│  launch.json + DAP Client   │
└──────────┬──────────────────┘
           │ TCP (port 50001)
┌──────────▼──────────────────┐
│  baker-link-env (ホスト)     │
│  ProbeRsDapServer            │
│    └─ probe-rs DAP Server    │
└──────────┬──────────────────┘
           │ USB (SWD/JTAG)
┌──────────▼──────────────────┐
│  Physical MCU (RP2040 etc.) │
└─────────────────────────────┘
```

### グローバルシングルトン

```rust
// main.rs で定義
static DAP_SERVER: OnceLock<Mutex<cmd::ProbeRsDapServer>> = OnceLock::new();

// どこからでもアクセス
crate::dap_server()  // → &'static Mutex<ProbeRsDapServer>
```

## ProbeRsDapServer の構造

```rust
pub struct ProbeRsDapServer {
    pub port: String,                              // リッスンポート
    shutdown: Option<CancellationToken>,            // グレースフルシャットダウン用
    handle: Option<std::thread::JoinHandle<()>>,    // ワーカースレッド
    pub status: DapServerStatus,                    // Running(port) | Stopped
}
```

## Procedure

### 1. DAP サーバーの起動フロー

```rust
impl ProbeRsDapServer {
    pub fn start(&mut self, tx: mpsc::Sender<String>) -> Result<(), String> {
        if self.status != DapServerStatus::Stopped {
            return Ok(());  // 二重起動防止
        }
        let port = self.parse_port()?;
        let shutdown = CancellationToken::new();
        let shutdown_task = shutdown.clone();

        // 別スレッドで Tokio ランタイムを作成し DAP サーバーを実行
        let handle = spawn_dap_server_thread(port, shutdown_task, tx);

        self.shutdown = Some(shutdown);
        self.handle = Some(handle);
        self.status = DapServerStatus::Running(port);
        Ok(())
    }
}
```

**重要:** DAP サーバーは専用スレッドで `tokio::runtime::Builder::new_current_thread()` を使って新しい Tokio ランタイム上で動く。メインの Dioxus ランタイムとは独立している。

### 2. DAP サーバーの停止フロー

```rust
pub fn stop(&mut self) -> bool {
    if self.status == DapServerStatus::Stopped {
        return false;
    }
    // 1. CancellationToken でシャットダウンを通知
    if let Some(shutdown) = self.shutdown.take() {
        shutdown.cancel();
    }
    // 2. JoinHandle をデタッチ（UIスレッドをブロックしない）
    if let Some(handle) = self.handle.take() {
        thread::spawn(move || {
            let _ = handle.join();
        });
    }
    self.status = DapServerStatus::Stopped;
    true
}
```

**必須パターン:**
- `CancellationToken` で安全にシャットダウンを通知
- `handle.join()` は別スレッドでデタッチ — UIスレッドブロック禁止
- `shutdown` と `handle` を `take()` で所有権を移動

### 3. probe-rs API 呼び出し

```rust
use probe_rs_tools::cmd::dap_server;

// 実際の DAP サーバー起動
dap_server::run_with_shutdown_on_port(
    port,           // u16: リッスンポート
    false,          // single_session: false = マルチセッション
    None,           // log_file: Option<PathBuf>
    offset,         // UtcOffset: ログタイムスタンプ用
    shutdown_task,  // CancellationToken
)
```

### 4. ログ連携

DAP サーバースレッドのログは `mpsc::Sender<String>` 経由で `DisplayBuffer` に送る:

```rust
fn spawn_dap_server_thread(
    port: u16,
    shutdown_task: CancellationToken,
    log_tx: mpsc::Sender<String>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // ランタイムエラー → log_tx で通知
        // DAP サーバーエラー → shutdown 状態を確認して適切にログ
        if shutdown_probe.is_cancelled() {
            let _ = log_tx.send("DAP server shutdown requested".to_string());
        } else {
            let _ = log_tx.send(format!("[ERROR] DAP server stopped: {error}"));
        }
    })
}
```

### 5. UI との連携 (AppAction)

```rust
// app.rs
AppAction::StartDap => {
    if let Ok(mut server) = crate::dap_server().lock() {
        let tx = crate::display_buffer().lock().ok()?.sender();
        match server.start(tx) {
            Ok(()) => {
                dap_running.set(true);
                crate::log_info(format!("DAP Server started on port {}", server.port));
            }
            Err(e) => {
                crate::log_error(e.clone());
                last_error.set(Some(e));
            }
        }
    }
}
AppAction::StopDap => {
    if let Ok(mut server) = crate::dap_server().lock() {
        if server.stop() {
            dap_running.set(false);
            crate::log_info("DAP Server stopped");
        }
    }
}
```

## Path Mapping (Docker ↔ Host)

Dev Container 内のパスとホストのパスが異なるため、VS Code の `launch.json` で `pathMappings` を設定する必要がある:

```json
{
    "pathMappings": [
        {
            "remoteRoot": "/workspaces/project",
            "localRoot": "${workspaceFolder}"
        }
    ]
}
```

**注意事項（既知の問題）:**
- probe-rs のブレークポイント照合はリクエストパスが相対の場合に DWARF の絶対パスとマッチしないことがある
- ホスト絶対パスマッピングを相対パスフォールバックより優先すべき
- 詳細は `/memories/repo/dap-path-matching.md` を参照

## Quality Checklist

- [ ] `CancellationToken` でグレースフルシャットダウンが実装されている
- [ ] `handle.join()` はUIスレッド外でデタッチされている
- [ ] 二重起動チェック (`status != Stopped`) がある
- [ ] ログは `mpsc::Sender<String>` 経由で `DisplayBuffer` に送っている
- [ ] エラー時に `shutdown.is_cancelled()` を確認して正常停止と異常停止を区別
- [ ] UI 側は `AppAction` 経由でのみ start/stop を発行
