# Changelog

All notable changes to this project will be documented in this file.

Entries are based on Git tags and commit history. Version headings use a
consistent `vX.Y.Z` format even where historical tag names differ. No v0.1.1
tag or package-version change was found; v0.2.1 is recorded as an untagged
development version.

## v0.2.8

### User-facing changes
- Added ELF cache controls to Advanced Settings: usage, file and active-file counts, configurable capacity, and deletion of unused artifacts.
- Added a persistent 1 GiB default cache limit with least-recently-used eviction of inactive artifacts, cleanup of interrupted uploads, and protection for artifacts used by active debug sessions.

### Bug fixes
- Preserved DAP availability after repeated connection checks and client disconnects, with regression tests for subsequent initialize requests.
- Allowed re-uploading an active project's ELF for debugger restarts while preserving cache accounting and active-artifact protection.
- Hardened log handling for short and non-ASCII messages.
- Improved debug server shutdown and restart handling, including Windows upload-listener wake-up and port-release checks.
- Fixed intermittent ELF upload shutdown errors during debug server restarts by verifying port release when the listener wake-up connection fails.

### Notes
- Expanded debugging guidance for RTT, reset/halt behavior, cache management, and waiting for shutdown before restarting.

## v0.2.7

### User-facing changes
- Moved debug server bind IP, DAP port, and upload port controls into Advanced Settings.
- Improved log presentation with timestamps and severity labels, and added log clearing.

### Bug fixes
- Added retry-based port availability checks and prevented starting the debug server while shutdown is still in progress.
- Preserved unformatted log messages instead of incorrectly splitting them as timestamped output.

## v0.2.6

### User-facing changes
- Added Doctor environment diagnostics for VS Code, Dev Containers, Docker, and host networking, with setup links, details, and a copyable report.
- Added persistent English/Japanese language selection for application messages and Help links, plus a Japanese README.
- Added automatic host-network configuration for managed template projects, using native Desktop DNS or a host-gateway override for supported local Linux Docker Engine setups.

### Notes
- Added the Baker Link VS Code extension as a submodule and updated the tutorial template integration.
- Expanded setup and troubleshooting documentation for supported Docker runtimes and unsupported remote/rootless configurations.

## v0.2.5

### User-facing changes
- Added direct opening of Dev Container workspaces in VS Code, including Dev Containers extension checks and waiting for Docker readiness.
- Improved project-generation errors and handling of existing destination folders.
- Changed the default bind IP to `0.0.0.0` for container reachability and removed client authentication from the ELF upload endpoint.

### Bug fixes
- Improved coordinated DAP/upload shutdown so the server remains in the stopping state until its worker threads finish.

### Notes
- Updated Dioxus dependencies and documented the matching Dioxus CLI version, 0.7.10.
- Documented trusted-network and firewall requirements for unauthenticated debug services.

## v0.2.4

### User-facing changes
- Added an ELF upload server and host-side artifact cache with project/profile validation, upload-size limits, and SHA-256 verification.
- Added `bakerlink://<project>/<profile>` artifact references for debugging container-built ELF files without host path mappings.
- Added the Baker Link tutorial template as a submodule.

### Bug fixes
- Loaded the bundled Tailwind stylesheet through the desktop application's custom header.

## v0.2.3

### User-facing changes
- Added Tailwind-based UI styling and connected debug probe detection.
- Added Linux support for VS Code and Rancher Desktop commands, plus Linux release packaging.
- Added configurable DAP server bind IP.

### Bug fixes
- Updated the embedded probe-rs DAP API call and fixed the stylesheet asset path to restore builds.
- Removed the stale probe-rs-espressif dependency that blocked builds.

### Notes
- Added development and release build optimization profiles.

## v0.2.2

### User-facing changes
- Refreshed the Dioxus desktop interface, project-generation flow, and logging components.
- Improved clipboard integration and Windows MSI packaging.

### Bug fixes
- Fixed macOS build failure caused by Rust compiler's inline module path resolution using `..` in `#[path]` attributes in `probe-rs-tools`. The virtual directories (`src/cmd/`, `src/util/`) are now created before the build step on macOS so that path traversal succeeds.
- Corrected project-history saving and improved macOS Docker command PATH handling and output validation.

### Notes
- Improved release build reliability with a committed lockfile, revised Dioxus CLI installation, and macOS dependency setup.

## v0.2.1

### User-facing changes
- Migrated the desktop interface from egui/eframe to Dioxus, with updated styling, embedded logo assets, project history, and Docker status handling.
- Added container-to-host path-mapping guidance to the debug server UI used at that time.

### Notes
- This package version appears in commit history but has no release tag.
- Returned the Rust toolchain to stable and reorganized desktop configuration and release packaging.

## v0.2.0

### User-facing changes
- Added build version display in the header using the current Git tag (v*.*.*) and commit hash.
- Added a "Start RD" button in the header when Docker is stopped (hidden when running).
- Kept the Rancher Desktop popup, and made its Start action reuse the same startup flow.
- Reduced UI stutter by moving Docker status checks off the UI thread.
- Switched probe-rs from an external command launch to an embedded integration.

### Notes
- External probe-rs warnings are silenced with line-level allows to keep other diagnostics visible.

## v0.1.5

### User-facing changes
- Added GitHub Actions build and release workflows.
- Added one-time startup initialization with Rancher Desktop startup result logging.

### Bug fixes
- Corrected Help URLs.
- Fixed macOS Rancher Desktop command lookup and startup by sourcing `.zshrc`.

## v0.1.4

### Bug fixes
- Corrected the project template repository URL.

### Notes
- Updated project documentation and license information.

## v0.1.3

### User-facing changes
- Added Rancher Desktop startup integration.
- Changed the information action to open the project's GitHub page.

### Bug fixes
- Fixed Rancher Desktop startup on Windows.

## v0.1.2

### User-facing changes
- Added duplicate-instance prevention.

### Bug fixes
- Fixed application icons, project paths, and project-history handling.

## v0.1.0

### User-facing changes
- Initial release with project creation, VS Code launching, probe-rs integration, and project history.
- Added application information and Windows icon support.

### Bug fixes
- Corrected probe-rs execution and VS Code launching on Windows, and suppressed extra probe-rs console windows.
