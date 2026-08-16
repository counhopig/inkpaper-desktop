# Inkpaper Desktop

PC configuration tool for the [Inkpaper NOTE4 firmware](../inkpaper) - a
cross-platform (Linux/Windows/macOS) native app built with Rust + `egui`
(no Node/webview dependency, single `cargo build` per platform).

This tool does **not** author content. It has two jobs:

- **Device tab**: push Wi-Fi credentials and server config to the physical
  device over USB serial or BLE, trigger a sync, check status. Talks the
  protocol documented in [`../inkpaper/docs/control-protocol.md`](../inkpaper/docs/control-protocol.md).
- **Server tab**: register devices and manage their alarms/todos against
  [`inkpaper-server`](../inkpaper-server)'s admin API. This is where actual
  content gets authored - the device just pulls it later over Wi-Fi.

## Build

```bash
cargo build --release
```

Produces a native binary at `target/release/inkpaper-desktop`. No
additional system dependencies beyond what `egui`/`eframe` need for a
window (X11/Wayland on Linux, native on Windows/macOS) and `btleplug`
needs for BLE (BlueZ via D-Bus on Linux, WinRT on Windows, CoreBluetooth
on macOS).

## Run

```bash
cargo run --release
```

Opens the GUI. Pick a USB serial port or use "Connect BLE" (scans for a
device advertising as `Inkpaper` - only visible while its BLE Pairing
screen is open), then use the Device tab to push Wi-Fi/server config, or
the Server tab (needs a running `inkpaper-server` URL + admin token) to
manage content.

There's also a headless CLI mode for scripting/testing without the GUI:

```bash
cargo run --release -- --status /dev/ttyACM0
```

Connects over USB, sends `get_status`, prints the parsed reply, exits.

## Status

USB transport verified against real hardware. BLE transport is
implemented against the same documented protocol but has not been tested
against a real device (no BLE adapter/peer was available when this was
built) - if something doesn't work, start there. See
[`../inkpaper/docs/project-status.md`](../inkpaper/docs/project-status.md)
for the full cross-repo status.
