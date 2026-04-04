# Changelog

All notable changes to this project will be documented in this file.

## v0.2.2

### Bug fixes
- Fixed macOS build failure caused by Rust compiler's inline module path resolution using `..` in `#[path]` attributes in `probe-rs-tools`. The virtual directories (`src/cmd/`, `src/util/`) are now created before the build step on macOS so that path traversal succeeds.

## v0.2.0

### User-facing changes
- Added build version display in the header using the current Git tag (v*.*.*) and commit hash.
- Added a "Start RD" button in the header when Docker is stopped (hidden when running).
- Kept the Rancher Desktop popup, and made its Start action reuse the same startup flow.
- Reduced UI stutter by moving Docker status checks off the UI thread.
- Switched probe-rs from an external command launch to an embedded integration.

### Notes
- External probe-rs warnings are silenced with line-level allows to keep other diagnostics visible.
