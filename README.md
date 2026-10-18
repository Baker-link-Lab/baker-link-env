# Baker link. Env

**[English](README.md) | [日本語版README](README.ja.md)**

An embedded Rust development assistant for Baker link. series boards.
Edit and build in a VS Code Dev Container, then debug physical MCUs through
probe-rs running on your computer. No host Rust toolchain is needed for normal use.

![Home screen](image/app_home.png)

## Quick Start

1. Install the app from [Releases](https://github.com/Baker-link-Lab/baker-link-env/releases).
2. Prepare [VS Code](#vscode), [Dev Containers](#dev-containers), and [Docker](#docker-cli).
3. Start Baker Link Env. **Doctor** checks your environment automatically.
   Fix any **Needs attention** items using **Read setup steps**, then **Run checks**.
4. Enter a project name in **Create Project**, click **Create**, and choose a folder.
   Wait for VS Code's container and its Baker Link extension to finish setting up.
5. Connect your debug probe and board. Click **Run** in the desktop app, then
   press **F5** in VS Code to build, upload the ELF, and debug.

Choose **English / 日本語** in the top bar to switch the app and Help language.
The selection is saved; resetting settings returns to English.
Commands, paths, device names, and external tool output stay unchanged.
Native dialog controls follow your OS language.

<a id="environment-setup"></a>
## Environment Setup

Doctor checks host tools and Docker compatibility, not projects, container
connections, or USB devices. It does not install software or change settings.
Commands have a 15-second timeout each.

| Result | What to do |
|---|---|
| Passed | This check succeeded. |
| Needs attention | Open **Read setup steps**, fix the issue, and run checks again. |
| Not checked | Fix the prerequisite check first. |

Start with the first failed item:
[VS Code](#vscode) · [Dev Containers](#dev-containers) ·
[Docker CLI](#docker-cli) · [Docker Engine](#docker-engine) ·
[Host networking](#host-networking).

Use **Back to Home** to return and **×** to close notifications.
If the problem remains, open **Details** or **Copy report**; review private
paths/host names before sharing. Online Help requires internet access and
documentation published to `main`.

<a id="vscode"></a>
### Set Up VS Code

1. Install [VS Code](https://code.visualstudio.com/download) and open it once.
2. Enable the `code` command:
   - **Windows:** select **Add to PATH** in the installer.
   - **macOS:** put VS Code in `/Applications` or `~/Applications`. For another
     location, use the Command Palette's **Shell Command: Install 'code' command in PATH**.
   - **Debian:** follow the [Linux installation guide](https://code.visualstudio.com/docs/setup/linux).
3. In a new host terminal, run `code --version`. After changing PATH, fully
   exit and reopen Baker Link Env, then run Doctor again.

If it still fails, check **Details** for permission errors or timeouts.

<a id="dev-containers"></a>
### Install Dev Containers

1. Open VS Code **on your computer**, not a container-connected window.
2. Open Extensions (`Ctrl+Shift+X`, or `Cmd+Shift+X` on macOS).
3. Install Microsoft's [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
   (`ms-vscode-remote.remote-containers`), then run Doctor again.

Use the same VS Code installation and default profile as the `code` CLI.
If extension listing fails, fix the VS Code check first.

<a id="docker-cli"></a>
### Set Up Docker

1. Install [Rancher Desktop](https://docs.rancherdesktop.io/getting-started/installation/)
   and select **dockerd (moby)**, not `containerd`. Docker Desktop is also
   supported; Debian can use local, rootful [Docker Engine](https://docs.docker.com/engine/install/debian/).
2. On Windows, complete the runtime's WSL 2 prerequisites. With Docker Desktop,
   use **Linux containers**.
3. Follow the runtime's CLI/PATH instructions. In a new host terminal, check
   `docker --version`, then fully reopen Baker Link Env and run Doctor again.

If unsure, use one runtime. A working CLI does not mean the engine is running.

<a id="docker-engine"></a>
### Docker Cannot Connect

1. Start your Docker runtime and wait until it is ready. Check `docker info`
   in a host terminal.
2. If it fails, use `docker context ls` to check the selected engine (`*`).
   Use `docker context use NAME` only with your intended runtime's context.
3. On Debian, follow the [post-installation guide](https://docs.docker.com/engine/install/linux-postinstall/)
   for socket permission errors. Docker group membership grants powerful host
   access; read the warning and log in again if required.
4. Run Doctor again. For malformed output or timeouts, check `docker info`
   and update/repair the runtime.

<a id="host-networking"></a>
### Supported Host Networking

| Runtime | Host connection |
|---|---|
| Rancher Desktop / Docker Desktop | Native `host.docker.internal` DNS |
| Local, rootful Docker Engine on Debian/Linux | `host.docker.internal:host-gateway` |

Windows containers, rootless native Docker, and remote native Docker are
outside automatic setup. Check `docker context ls` and `docker info` rather
than guessing IP addresses or disabling the firewall.

Opening a template project through Baker Link Env updates
`.docker/compose.host.yaml`. After changing hosts/runtimes, reopen through the
app and use **Dev Containers: Rebuild Container**; save work first because
recreation stops container processes. Do not add unconditional `host-gateway`
to the base Compose file on Desktop runtimes.
For manual opening, see the [template guide](template/bakerlink_tutorial_template/README.md).

## Debugging and Connection Problems

- Default **Bind IP**: `0.0.0.0`; **DAP**: `50001`; **Upload**: `50002`.
- Click **Run** on the host before debugging. The extension does not start
  host services automatically.
- If ports differ, update `bakerLink.dapServer` and `bakerLink.uploadUrl` in
  `.vscode/launch.json`. Keep `remoteServerMode: false` and
  `programBinary: "bakerlink://<project>/<profile>"`.
- For release/examples, match the build task, `bakerLink.elf`, and artifact profile.
- Use **Baker Link: Check Host Connection** for TCP checks and the **Baker Link Env**
  output channel for errors. Port checks do not verify USB/MCU readiness.
- The template uses `haltAfterReset: false` to run after flashing and debugger
  configuration. RTT appears in a terminal once firmware initializes it.
  With `haltAfterReset: true`, press Continue (**F5**) after the initial stop.
  A successful ELF upload only confirms host cache storage, not target flashing.

**Security:** DAP and ELF upload are unauthenticated. `0.0.0.0` listens on all
host interfaces; restrict access to trusted networks with firewall rules.
Loopback-only `127.0.0.1` is not suitable for normal container access.
No host path mapping or `LOCAL_WORKSPACE_FOLDER` is required.

<details>
<summary>Existing projects and extension migration</summary>

The template installs `baker-link-lab.baker-link-env` inside the container.
Existing containers need **Rebuild Container** or manual extension installation.
Use debugger type `baker-link-debug`. For old projects, move endpoints from
`.bakerlink/connection.json` to `bakerLink` in `.vscode/launch.json`, keep a
build-only pre-launch task, and remove shell-upload tasks and `.bakerlink`.
See the [extension guide](external/baker-link-env-vscode/README.md).

Projects opened from **History** use the same container-opening behavior.
Projects without a Dev Container configuration open locally.
Container configs need an absolute `workspaceFolder`;
`${localWorkspaceFolderBasename}` is supported. The app can start Rancher
Desktop and wait up to 60 seconds when opening projects.

</details>

<details>
<summary>Building and extension development</summary>

Initialize submodules with `git submodule update --init --recursive`.
Use Dioxus CLI **0.7.10**, matching the pinned dependencies:

```sh
cargo run
cargo install dioxus-cli --version 0.7.10 --locked --force
dx bundle --release --package-types deb
```

Use `msi` on Windows or `dmg` on macOS. Update CLI and dependencies together.
For extension development (VS Code >= 1.116.0, Node >= 22):

```sh
cd external/baker-link-env-vscode
npm ci
npm test
npm run package
```

Install the VSIX in the project's Dev Container using **Extensions: Install from VSIX...**.

</details>

### macOS: "Damaged App" Error

Only for an app from the trusted official release, remove its quarantine flag:

```sh
xattr -d com.apple.quarantine "/Applications/Baker link. Env.app"
```

## License

Copyright (c) 2024 Baker-Tanaka. [MIT](LICENSE).

[Support Baker link.](https://www.buymeacoffee.com/Bakerlink.Lab)
