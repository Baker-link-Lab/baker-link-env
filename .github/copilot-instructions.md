# Project Guidelines

## Overview

Baker Link Env — 組込みRust開発環境の自動セットアップを行うデスクトップアプリ。
VS Code Dev Container、probe-rs DAPサーバー、物理マイコン（RP2040等）のブリッジとして動作する。

## Tech Stack

- **Language**: Rust (Edition 2021, stable toolchain)
- **UI**: Dioxus 0.7.1 Desktop — RSX記法による宣言的UI
- **Async**: Tokio ランタイム
- **Debug**: probe-rs DAP server (`external/probe-rs/` サブモジュール)
- **Scaffolding**: cargo-generate によるテンプレートプロジェクト生成
- **Platform**: Windows / macOS クロスプラットフォーム

## Architecture

```
src/
  main.rs       — Dioxus初期化、ウィンドウ設定、トレイ、グローバルシングルトン
  app.rs        — メインUIコンポーネント(App)、AppAction、リアクティブ状態管理
  cmd.rs        — DAPサーバー管理(ProbeRsDapServer)、外部コマンド実行
  helpers.rs    — アイコン、CSS補助、ログパース
  logger.rs     — DisplayBuffer(mpscチャネル+リングバッファ)
  parameter.rs  — 定数、ビルド時Git情報
  settings.rs   — AppSettings JSON永続化(~/.config/baker-link-env/)
assets/
  main.css      — ダークテーマ、CSS変数によるデザインシステム
external/
  probe-rs/     — probe-rsサブモジュール（DAPサーバー実装）
```

## Code Style

- Dioxus の RSX マクロで UI を記述する。HTML風の構文を使い、`rsx! { ... }` ブロック内に記述する
- 状態管理には Dioxus の `use_signal` を使用する
- 副作用ディスパッチは `AppAction` enum + `use_coroutine` パターンに従う
- クロスプラットフォーム分岐は `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` で行う
- エラーハンドリングには `anyhow::Result` を使用する
- ロギングは `DisplayBuffer` のチャネル経由で行い、UIスレッドをブロックしない

## Build and Test

```sh
# 開発実行
cargo run
# または
dx serve --platform desktop

# ビルド
cargo build --release

# Lint
cargo clippy
cargo fmt --check
```

## Conventions

- CSS は `assets/main.css` に集約。CSS変数プレフィックスは `--bkl-`（例: `--bkl-orange`, `--bkl-green`）
- 設定ファイルは `~/.config/baker-link-env/settings.json` に保存
- `build.rs` で Git ハッシュとタグを環境変数に埋め込む（`GIT_HASH`, `GIT_TAG`）
- DAPサーバーは `CancellationToken` によるグレースフルシャットダウンを徹底する
- `OnceLock` をグローバルシングルトン（`DISPLAY_BUFFER`, `DAP_SERVER`）に使用
- 外部コマンドの Windows 実行は `cmd /C` または PowerShell、macOS は `sh -c` を使う
