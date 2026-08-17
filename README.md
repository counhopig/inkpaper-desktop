# Inkpaper Desktop

PC configuration tool for the [Inkpaper NOTE4 firmware](../inkpaper) - a
cross-platform (Linux/Windows/macOS) native app built with:

- **Tauri 2** - native window, system menus, file dialogs, OS notifications
- **Vue 3 + TypeScript** - the in-window UI (Composition API + Pinia)
- **Rust** - device transport (USB serial, BLE), server admin HTTP,
  command/event plumbing

It does **not** author content. It has four jobs:

- **Overview** - glanceable device + server state and setup progress.
- **Device** - push Wi-Fi credentials, sync server URL + device token,
  and timezone to the physical device over USB serial or BLE; trigger
  a sync; check status. Talks the protocol documented in
  [`../inkpaper/docs/control-protocol.md`](../inkpaper/docs/control-protocol.md).
- **Content** - register devices and manage their alarms/todos against
  [`inkpaper-server`](../inkpaper-server)'s admin API. This is where
  actual content gets authored; the device just pulls it later over
  Wi-Fi.
- **Logs** - real-time diagnostics, mirrored to disk in the platform
  log directory (see [Logging](#logging)).

## Repository layout

```text
inkpaper-desktop/
├── src/                       # Rust (Tauri + transport + server client)
│   ├── main.rs                # CLI dispatch and Tauri launch
│   ├── desktop.rs             # Tauri builder + invoke_handler registration
│   ├── state.rs               # AppState + LinkState + Usb/Ble handles
│   ├── error.rs               # AppError + error-code catalog
│   ├── protocol.rs            # Wire types shared with the firmware
│   ├── server.rs              # HTTP client for inkpaper-server
│   ├── transport/             # USB and BLE workers
│   └── commands/              # Tauri commands (one file per surface)
├── src-ui/                    # Vue 3 + TS + Pinia
│   ├── App.vue                # Root: shell + page routing
│   ├── pages/                 # OverviewPage, DevicePage, ContentPage, LogsPage
│   ├── components/            # AppShell, Sidebar, TopBar, Frame, Button, ...
│   ├── stores/                # Pinia: device, server, logs
│   ├── lib/                   # commands.ts (Tauri wrapper), types.ts, ...
│   └── styles/                # tokens.css + global/layout/components.css
├── icons/                     # Bundle icons
├── index.html                 # Vite entry
├── package.json               # Front-end deps + scripts
├── vite.config.ts             # Vite config
├── tsconfig.json              # TS config (strict)
├── tauri.conf.json            # Window + bundle config
├── Cargo.toml                 # Rust deps
└── build.rs                   # Tauri build script
```

## Develop

```bash
# one-time
npm install

# full Tauri dev mode (hot-reloads Vue, rebuilds Rust on change)
npm run tauri dev
```

## Front-end checks

```bash
npm run build   # vue-tsc --noEmit && vite build
```

## Rust tests

```bash
cargo test
```

Covers the JSON wire format, URL normalisation, secret redaction, and
the `AppError` serialisation shape. The Vue side is currently
type-checked only (no Vitest; tests would be added if the surface
grows).

## Release build

```bash
npm run tauri build
```

Produces a signed `.app` bundle on macOS, an MSIX/`.exe` on Windows,
and `.deb`/`.AppImage` on Linux (see Tauri's default bundler config).

## CLI

The same binary runs as a headless tool for scripting. CLI arguments
do **not** launch the Tauri window - they exit after one command:

```bash
inkpaper-desktop --ble-scan                        # true/false if Inkpaper advertises
inkpaper-desktop --ble-list                        # full btleplug peripheral dump
inkpaper-desktop --status /dev/cu.usbmodem1101     # USB status (default timeout 35s)
inkpaper-desktop --status /dev/cu.usbmodem1101 60  # custom timeout
inkpaper-desktop --sync   /dev/cu.usbmodem1101     # USB sync (default timeout 45s)
inkpaper-desktop --sync   /dev/cu.usbmodem1101 90  # custom timeout
```

The ESP32-S3 USB Serial/JTAG port may reset the board on open, so
status/sync timeouts are deliberately generous. Pass a longer timeout
when scripting against a freshly booted board.

## macOS permissions

The first time you run the GUI on macOS, the system will prompt for:

- **Bluetooth** - required for BLE scanning/connection. Denying it
  disables the BLE section of the Device page; USB continues to work.
- **USB serial** - no special permission needed for the
  `/dev/cu.usbmodem*` devices exposed by the ESP32-S3 USB
  Serial/JTAG peripheral on most macOS versions. If a Corporate-managed
  profile blocks raw USB access, run from a Terminal session that has
  been granted the entitlement.

## Design language

Both the Desktop and the Server UI are intentionally built to feel like
the e-ink screen of the device itself: paper-grey surfaces, ink-black
rules, no large colour fills, no saturated accents, hierarchy carried
by line weight and spacing rather than colour. Status is conveyed with
glyphs (`○ ◉ △ ✓ ✕`) so meaning does not depend on hue. The design
system is encoded in `src-ui/styles/tokens.css`; extending it means
adding a new CSS variable there rather than scattering hex codes
through components.

## Logging

Logs are written to `inkpaper-desktop-<unix-epoch>.log` inside a
platform data directory on every launch:

| OS      | Directory                                        |
| ------- | ------------------------------------------------ |
| macOS   | `~/Library/Logs/inkpaper-desktop/`               |
| Windows | `%LOCALAPPDATA%\inkpaper-desktop\logs\`           |
| Linux   | `~/.local/share/inkpaper-desktop/logs/`           |

The directory is deliberately outside the project tree: `tauri dev`
watches the project root, so a log file inside it would restart the
app in an infinite loop as every flush touched the watched directory.

Sensitive values are redacted in the on-disk file (see `redact_secret`
in `src/commands/logs.rs`):

| Source            | Examples                                          |
| ----------------- | ------------------------------------------------- |
| Logged verbatim   | USB/BLE command names, replies, error codes       |
| Redacted          | Wi-Fi passwords, Admin Tokens, Device Tokens      |

The Logs page reads via the `device-log` Tauri event and never polls
on a timer. "Open log folder" launches the OS file manager; "Export
log" copies the current file into `~/Downloads/`.
