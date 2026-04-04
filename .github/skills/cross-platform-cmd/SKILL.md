---
name: cross-platform-cmd
description: "クロスプラットフォーム外部コマンド実行。Use when: Windows/macOS対応コマンド追加、外部プロセス起動、cfg分岐、cmd.rs修正、シェルコマンド実行"
---

# クロスプラットフォーム コマンド実行

## When to Use
- 新しい外部コマンド・プロセス起動を追加するとき
- Windows / macOS 両対応の処理を書くとき
- `cmd.rs` に機能を追加するとき

## Architecture

`cmd.rs` はすべての外部コマンド実行を集約する。パブリック関数は OS 非依存の API を提供し、内部で `#[cfg]` 分岐する。

```
pub fn my_command(args)    ← OS非依存のパブリックAPI
├── #[cfg(windows)] my_command_windows(args)
└── #[cfg(macos)]   my_command_macos(args)
```

## Procedure

### 1. パブリックAPI関数を定義

```rust
pub fn my_command(param: &str) -> Result<std::process::Output, std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        my_command_windows(param)
    }
    #[cfg(target_os = "macos")]
    {
        my_command_macos(param)
    }
}
```

**戻り値パターン:**
- 出力が必要: `Result<std::process::Output, std::io::Error>`
- 成功/失敗のみ: `Result<(), String>`
- Bool判定: `Result<bool, String>`

### 2. Windows 実装

```rust
#[cfg(target_os = "windows")]
fn my_command_windows(param: &str) -> Result<std::process::Output, std::io::Error> {
    let path = std::env::var("PATH").unwrap_or_default();
    std::process::Command::new("cmd")
        .args(["/C", "some-command", param])
        .env("PATH", &path)
        .creation_flags(CREATE_NO_WINDOW)  // コンソールウィンドウを非表示
        .output()
}
```

**Windows 必須ルール:**
- `CREATE_NO_WINDOW` (0x08000000) を `.creation_flags()` で設定 — GUIアプリなのでコンソール窓を出さない
- `std::os::windows::process::CommandExt` を `#[cfg(target_os = "windows")]` 付きでインポート
- PATH環境変数を明示的に渡す: `.env("PATH", std::env::var("PATH").unwrap_or_default())`
- `code.cmd`（VS Code）のように `.cmd` 拡張子が必要な場合がある

### 3. macOS 実装

```rust
#[cfg(target_os = "macos")]
fn my_command_macos(param: &str) -> Result<std::process::Output, std::io::Error> {
    let home_dir = std::env::var("HOME").unwrap();
    let zshrc_path = format!("{}/{}", home_dir, ZSH_PROFILE);  // ".zshrc"
    
    std::process::Command::new("zsh")
        .arg("-c")
        .arg(format!("source {} && some-command {}", zshrc_path, param))
        .output()
}
```

**macOS 必須ルール:**
- `zsh -c` でシェル経由実行し、`source ~/.zshrc` で PATH を読み込む
- VS Code は `open -a "Visual Studio Code"` で起動
- GUI アプリケーション起動は `open -a` を使用

### 4. UIからの呼び出し

UIスレッドをブロックしないため、必ず `std::thread::spawn` + `oneshot::channel` パターンを使う:

```rust
// app.rs の AppAction handler 内
AppAction::MyAction(param) => {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let param_clone = param.clone();
    std::thread::spawn(move || {
        let _ = tx.send(cmd::my_command(&param_clone));
    });
    match rx.await {
        Ok(Ok(output)) => {
            crate::log_info(format!("Command succeeded: {:?}", output.status));
        }
        Ok(Err(e)) => {
            crate::log_error(format!("Command failed: {}", e));
            last_error.set(Some(format!("Command failed: {}", e)));
        }
        Err(_) => {
            crate::log_error("Command channel closed");
        }
    }
}
```

**Fire-and-forget（結果不要の場合）:**
```rust
std::thread::spawn(|| { let _ = cmd::start_rd(); });
```

## Existing Patterns in cmd.rs

| 関数 | 用途 | 戻り値 |
|---|---|---|
| `open_vscode(path)` | VS Code でフォルダを開く | `Result<Output, io::Error>` |
| `start_rd()` | Rancher Desktop 起動 | `Result<(), String>` |
| `generate_project(name, path)` | cargo-generate でプロジェクト生成 | `anyhow::Result<PathBuf>` |
| `is_docker_running()` | Docker デーモンの稼働確認 | `Result<bool, String>` |

## Quality Checklist

- [ ] パブリック関数は OS 非依存、内部で `#[cfg]` 分岐
- [ ] Windows: `CREATE_NO_WINDOW` フラグ設定済み
- [ ] Windows: PATH 環境変数を明示的に渡している
- [ ] macOS: `zsh -c "source ~/.zshrc && ..."` パターンを使用
- [ ] UI 呼び出しは `thread::spawn` + `oneshot::channel` で非ブロッキング
- [ ] エラーは `crate::log_error()` で記録し、`last_error` signal に反映
