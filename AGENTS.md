# AGENTS.md — Inkpaper Desktop

PC config tool for the Inkpaper NOTE4 firmware (`../inkpaper`), one of three repos in this workspace (`inkpaper` firmware, `inkpaper-server` backend, this repo). English Tauri 2 app: Rust backend + Vue 3/Pinia frontend. Device wire protocol contract lives in `../inkpaper/docs/control-protocol.md`; server admin API in `../inkpaper-server`.

## Layout quirk

- `src/` = **Rust** (Tauri commands, USB/BLE transport, server HTTP client). `src-ui/` = **Vue** (pages/components/stores/lib/styles). Frontend is NOT in `src/`.
- Tauri commands are registered in `src/desktop.rs` (`invoke_handler`). A new command touches: the command fn, `desktop.rs`, and a typed wrapper in `src-ui/lib/commands.ts`.
- Wire types are hand-mirrored, no codegen: Rust structs use `#[serde(rename_all = "camelCase")]`, mirrored as camelCase interfaces in `src-ui/lib/types.ts`. Keep both sides in sync manually.
- `list_content` is the single alarms+todos endpoint; `list_alarms`/`list_todos` commands were deliberately removed as dead — do not re-add.

## Commands

```bash
npm run tauri dev      # dev app (vite fixed port 1420, strictPort)
npm run build          # vue-tsc --noEmit && vite build  — the only frontend check (no test framework)
cargo test             # Rust unit tests (protocol/error/logs)
cargo clippy --all-targets   # lint — this repo keeps it at zero warnings
npm run tauri build    # release bundle
```

Verification order after changes: `cargo clippy --all-targets` → `cargo test` → `npm run build`. Commit messages: conventional, lowercase type (`feat(ui):`, `fix(device):`, `refactor!:`), English.

## Gotchas

- **Log dir must stay outside the project tree** (`~/Library/Logs/inkpaper-desktop` on macOS): `tauri dev` watches the project root, so a log file inside it triggers an infinite dev-server restart loop. Logs mirror to stderr + `device-log` Tauri event; secrets go through `redact_secret` (never log Wi-Fi passwords / admin / device tokens verbatim).
- Frontend `invoke` never throws: `lib/commands.ts` `wrap()` converts rejections to `Result<T, AppError>`; error codes come from the catalog in `src/error.rs` (e.g. `SERVER_UNAUTHORIZED`, `INVALID_INPUT`).
- Events: Rust emits `connection-changed`, `sync-finished`, `device-log`; Pinia stores (`src-ui/stores/`) are the only subscribers.
- Same binary runs headless: `inkpaper-desktop --status <port> [timeout]`, `--sync`, `--ble-scan`, `--ble-list` — these never launch the window. Opening the ESP32-S3 USB serial port may reset the board, hence generous default timeouts (35/45s).
- macOS: first GUI run prompts for Bluetooth permission; deny only disables BLE, USB still works.
- Design language: "paper + ink" e-ink aesthetic. Extend styles via `src-ui/styles/tokens.css` variables, never scatter hex codes in components; status via glyphs (`○ ◉ △ ✓ ✕`), not color.
- Ignored/generated: `dist/`, `target/`, `gen/`, `node_modules/`, `logs/*.log` — never commit.
