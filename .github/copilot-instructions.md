# Baker link. Env

Baker link. Env is a Rust 2021 / Dioxus 0.7.10 desktop app that scaffolds embedded Rust projects, opens them in VS Code Dev Containers, and connects container debugging to host `probe-rs`.

## Build, test, and lint

Initialize submodules before building from a fresh checkout:

```sh
git submodule update --init --recursive
```

Use Dioxus CLI 0.7.10 to match the pinned Dioxus dependencies:

```sh
cargo install dioxus-cli --version 0.7.10 --locked --force
```

Common commands:

```sh
cargo run
dx serve --platform desktop
cargo build --release
cargo test
cargo test artifact::tests::stores_verified_artifact -- --exact
cargo clippy
cargo fmt --package baker-link-env -- --check
```

The focused test command runs one unit test by its full Rust test name. Tests live in `#[cfg(test)]` modules alongside the code they cover.

CI regenerates the bundled Tailwind stylesheet before formatting, tests, build, and packaging:

```sh
npx --yes tailwindcss@3.4.17 -i input.css -o assets/tailwind.css --minify
```

Bundle installers with Dioxus CLI, for example `dx bundle --release --package-types msi` on Windows, `dmg` on macOS, or `deb` on Linux. Linux builds need the GTK/WebKit and related system packages listed in `CLAUDE.md`; non-Windows builds also need the empty `external/probe-rs/probe-rs-tools/src/cmd` and `src/util` directories for probe-rs inline module resolution (CI creates them).

## Architecture

- `src/main.rs` initializes Dioxus Desktop, window assets, logging, and the process-wide `DAP_SERVER` singleton. `src/app.rs` owns the UI signals and dispatches side effects through one `AppAction` coroutine.
- `src/cmd.rs` is the boundary for external integrations: project generation, VS Code and Dev Container opening, Rancher Desktop/Docker, probe detection, and the `ProbeRsDapServer` lifecycle. Keep blocking work off UI event handlers; the coroutine delegates it to worker threads and returns results through Tokio oneshot channels.
- Starting the host debug services launches `probe-rs` DAP and the ELF upload server on separate listener threads. Both use a shared `CancellationToken`; the server remains `Stopping` until its worker threads have joined. The default ports are 50001 (DAP) and 50002 (upload).
- `src/upload.rs` validates upload requests and delegates storage to `src/artifact.rs`, which validates project/profile IDs, enforces the size limit and SHA-256 checksum, and stores artifacts in the OS cache. The VS Code extension uploads the container-built ELF before connecting to DAP; artifact references use `bakerlink://<project>/<profile>`, not host path mappings or `LOCAL_WORKSPACE_FOLDER`.
- `src/settings.rs` persists server settings, project history, and splash state as JSON under the OS config directory. `src/logger.rs` provides the `DisplayBuffer` channel used to relay background logs to the UI; the UI polls it every 300 ms. Use `crate::log_info` and `crate::log_error` for UI-visible messages.
- UI markup is Dioxus RSX in `src/app.rs`; Tailwind sources are `input.css` and `tailwind.config.js`, and the app loads the generated `assets/tailwind.css`. Keep custom component styles in the existing `@layer components` and use the `bkl-*` theme tokens.

## Repository conventions

- Use `#[cfg(target_os = "windows")`, `macos`, or `linux` for platform-specific external-command behavior. Windows child commands must use `CREATE_NO_WINDOW`; macOS Rancher Desktop commands source `~/.zshrc` so `rdctl` is available. Keep the public command API platform-neutral in `src/cmd.rs`.
- Preserve serde defaults when changing settings so existing settings files remain readable. Save settings through `settings::save` rather than writing the file directly.
- The DAP and ELF upload listeners do not authenticate clients. They default to binding `0.0.0.0` for container reachability; keep firewall/trusted-network requirements in mind when changing bind behavior.
- Direct Dev Container opening expects an explicit absolute `workspaceFolder`; `${localWorkspaceFolderBasename}` is the supported substitution. See `README.md` for end-user setup, connection, and troubleshooting details.
- Keep the Dioxus CLI version in sync with the pinned Dioxus dependencies and the release workflow when changing Dioxus versions.
