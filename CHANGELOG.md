# Changelog

All notable changes to **inkpaper-desktop** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-20

### Added
- Logs page shows a drafting-grid console with a device mini-screen and
  stat blocks.

### Fixed
- Log timestamps are stored as `u64` to avoid event serialization issues.
- The in-memory log buffer is resnapshotted when the Logs page opens so
  it always reflects the latest entries.
- Removed the background coordinate grid lines on the paper background,
  keeping only the faint dot grain.

### Changed
- The release workflow builds and **publishes** macOS/Windows/Ubuntu
  installers automatically on a `v*` tag (no manual draft handling).

## [0.1.0] - 2026-08-20

### Added
- Initial release: Tauri 2 + Vue 3 + TypeScript rewrite of the desktop
  configuration tool.
- Four pages: Overview (device/server state), Device (USB/BLE connect,
  Wi-Fi/server/timezone config, Sync Now), Content (register devices and
  author alarms/todos against `inkpaper-server`'s admin API), and Logs
  (real-time diagnostics with secret redaction).
- USB serial and BLE transports talking the firmware's control protocol.
- Headless CLI mode: `--status <port>`, `--sync <port>`, `--ble-scan`,
  `--ble-list`.
- Platform log directories with Wi-Fi passwords / admin / device tokens
  redacted on disk.
- Unified "paper + ink" e-ink design language.
- Apache-2.0 license and open-source README.
