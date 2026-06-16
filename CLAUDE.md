# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

**Baker link. Env** is a Dioxus desktop app (Rust) that automates embedded Rust development environment setup. It bridges VS Code Dev Containers (Docker) with a `probe-rs` DAP server running on the host, enabling in-container debugging of physical MCUs (RP2040, etc.) over SWD/JTAG.

## Commands

```sh
# Run in development
cargo run
# or via Dioxus CLI
dx serve --platform desktop

# Build release
cargo build --release

# Bundle installer (outputs to dist/)
dx bundle --release --package-types deb   # Linux
dx bundle --release --package-types dmg   # macOS
dx bundle --release --package-types msi   # Windows

# Lint
cargo clippy
cargo fmt --check
```

There are no automated tests in this project.

### Linux System Dependencies (for first-time build)

```sh
sudo apt-get install -y \
  pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libxdo-dev libudev-dev
```

On macOS: `brew install pkg-config libgit2`

### probe-rs Submodule Fix

On non-Windows, the inline module path resolution requires empty dirs to exist:

```sh
mkdir -p external/probe-rs/probe-rs-tools/src/cmd external/probe-rs/probe-rs-tools/src/util
```

This is a known build quirk — the CI workflow runs this step automatically.

## Architecture

```
src/
  main.rs       — Entry point: Dioxus window config, OnceLock globals (DAP_SERVER, DISPLAY_BUFFER)
  app.rs        — Root App component, AppAction enum, all reactive state (use_signal)
  cmd.rs        — ProbeRsDapServer struct, external command wrappers (VS Code, Rancher Desktop, cargo-generate)
  logger.rs     — DisplayBuffer: mpsc channel + Vec<String> ring, log_info/log_error helpers
  settings.rs   — AppSettings JSON persistence at ~/.config/baker-link-env/settings.json
  parameter.rs  — Constants (APP_NAME, template URL) + build_version_label() using GIT_HASH env var
  helpers.rs    — CSS class helpers, window icon loading, base64 logo data URI
assets/
  main.css      — Dark theme; CSS variables prefixed --bkl- (e.g. --bkl-orange, --bkl-green)
external/
  probe-rs/     — Git submodule; provides probe_rs_tools::cmd::dap_server::run_with_shutdown_on_port
```

## Key Patterns

**State and side-effects**: All UI signals are `use_signal`. Side-effects (start/stop DAP, open VS Code, start Docker) are dispatched as `AppAction` variants through a single `use_coroutine` in `app.rs`. Never call blocking operations directly from event handlers — always route through the coroutine or `std::thread::spawn` + oneshot channel.

**Globals**: `DAP_SERVER` and `DISPLAY_BUFFER` are `OnceLock<Mutex<T>>` singletons in `main.rs` / `logger.rs`. Access via `crate::dap_server()` and `crate::display_buffer()`.

**DAP server lifecycle**: `ProbeRsDapServer::start()` spawns a thread with its own `current_thread` Tokio runtime. Shutdown uses `CancellationToken` — always cancel via `stop()`, never kill the thread directly.

**Platform branching**: Use `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "linux")]`. Windows external commands need `CREATE_NO_WINDOW` flag; macOS needs to source `~/.zshrc` to get PATH with `rdctl`.

**Logging to UI**: Call `crate::log_info()` / `crate::log_error()`. These send to `DisplayBuffer`'s mpsc channel. The UI polls the buffer every 300ms via `use_future`.

**Settings persistence**: `settings::load()` / `settings::save()` — JSON at `~/.config/baker-link-env/settings.json`. `build.rs` embeds `GIT_HASH` via `cargo:rustc-env` at compile time.

## CI / Release

Releases are triggered by pushing a `v*.*.*` tag. The workflow builds on Windows (MSI), macOS (DMG), and Linux (DEB) and uploads to GitHub Releases. Requires `submodules: recursive` checkout.
