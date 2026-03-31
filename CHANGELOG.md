# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Full UI redesign — refreshed layout and visual style across all screens.
- New onboarding flow to guide first-time users through setup.
- Improved project creation wizard with template previews.

### Changed
- Upgraded to Dioxus 0.7 for a faster and more responsive desktop experience.
- Streamlined the main navigation to reduce clicks for common tasks.

### Fixed
- Various UI rendering inconsistencies on macOS and Windows resolved.

---

## [0.2.0] - 2025-04-01

### Added
- Build version display in the header showing the current Git tag (`v*.*.*`) and commit hash.
- "Start RD" button in the header that appears when Docker is stopped (hidden when running).
- Rancher Desktop popup with a Start action that reuses the same startup flow.

### Changed
- Docker status checks moved off the UI thread to reduce stutter.
- probe-rs switched from an external command launch to an embedded integration.

### Notes
- External probe-rs warnings are silenced with line-level `allow` attributes to keep other diagnostics visible.

---

## [0.1.5] - 2024-12-01

### Added
- Initial public release of Baker link. Env.
- Project creation with embedded Dev Container templates for Rust embedded development.
- probe-rs DAP Server integration for seamless debugging.
- Docker / Rancher Desktop status detection and launch support.
- Installer packages for Windows (NSIS) and macOS (DMG).

---

[Unreleased]: https://github.com/Baker-link-Lab/baker-link-env/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Baker-link-Lab/baker-link-env/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/Baker-link-Lab/baker-link-env/releases/tag/v0.1.5
