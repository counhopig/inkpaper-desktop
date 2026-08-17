//! Tauri commands for talking to the Inkpaper device over USB or BLE.
//!
//! The two phases of every command are split so the application-wide
//! `state.link` mutex is *released* before the slow `recv_timeout()`
//! happens:
//!
//! 1. Lock the link just long enough to push the command onto the
//!    worker's `mpsc::Sender`. Drop the guard immediately.
//! 2. Lock the per-link `event_rx` mutex, wait up to the deadline, drop.
//!    Other commands (BLE scans, log reads, server CRUD) can run while
//!    we're waiting.
//!
//! The transport-level workers run on plain OS threads with blocking
//! `mpsc` channels, so the slow `recv_timeout` here is a regular
//! blocking call wrapped in `tauri::async_runtime::spawn_blocking` at
//! each command entry point - the UI thread is never blocked.
//!
//! See `state::AppState` for the design rationale and `protocol` for
//! the wire format.

use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::desktop::SharedState;
use crate::error::AppError;
use crate::protocol::{Command, Reply};
use crate::state::{AppState, BleHandle, LinkState, UsbHandle};
use crate::transport::{ble::BleLink, usb::UsbLink};

const DEVICE_CMD_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCommandResult {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<DeviceStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub wifi_configured: bool,
    pub server_configured: bool,
    pub wifi_connected: bool,
}

fn result_from_reply(reply: Reply) -> DeviceCommandResult {
    match reply {
        Reply::Ok => DeviceCommandResult {
            kind: "ok".into(),
            message: "Device accepted the command".into(),
            status: None,
        },
        Reply::Status {
            wifi_configured,
            server_configured,
            wifi_connected,
        } => DeviceCommandResult {
            kind: "status".into(),
            message: "Device status received".into(),
            status: Some(DeviceStatus {
                wifi_configured,
                server_configured,
                wifi_connected,
            }),
        },
        Reply::Error { message } => DeviceCommandResult {
            kind: "error".into(),
            message,
            status: None,
        },
    }
}

/// Send `command` to the device and wait up to `DEVICE_CMD_TIMEOUT` for
/// a `Reply`. Synchronous - it blocks for up to 45s on the worker's
/// mpsc receiver. Wrap in `spawn_blocking` at the command entry point.
fn send_and_wait(state: &AppState, command: Command) -> Result<DeviceCommandResult, AppError> {
    let deadline = Instant::now() + DEVICE_CMD_TIMEOUT;

    // Log first - the command is consumed by `handle.send` below, so
    // we can't borrow it for logging afterwards.
    state.logs.info(
        "device",
        format!(
            "→ {}",
            serde_json::to_string(&command).unwrap_or_else(|_| "<unprintable>".into())
        ),
    );

    enum Phase {
        Usb,
        Ble,
    }
    let phase = {
        let mut guard = state
            .link
            .lock()
            .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
        match &mut *guard {
            LinkState::Disconnected => return Err(AppError::device_not_connected()),
            LinkState::Usb(handle) => {
                handle.send(command)?;
                Phase::Usb
            }
            LinkState::Ble(handle) => {
                handle.send(command)?;
                Phase::Ble
            }
        }
    };

    loop {
        if Instant::now() >= deadline {
            return Err(AppError::device_timeout());
        }
        let tick = {
            let mut guard = state
                .link
                .lock()
                .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
            match &mut *guard {
                LinkState::Disconnected => return Err(AppError::device_not_connected()),
                LinkState::Usb(handle) => match phase {
                    Phase::Usb => Some(TickEvent::Usb(
                        handle.recv_timeout(Duration::from_millis(250)),
                    )),
                    Phase::Ble => return Err(AppError::device_not_connected()),
                },
                LinkState::Ble(handle) => match phase {
                    Phase::Ble => Some(TickEvent::Ble(
                        handle.recv_timeout(Duration::from_millis(250)),
                    )),
                    Phase::Usb => return Err(AppError::device_not_connected()),
                },
            }
        };

        let Some(tick) = tick else { continue };
        match tick {
            TickEvent::Usb(Ok(crate::transport::usb::UsbEvent::Reply(reply))) => {
                let result = result_from_reply(reply.clone());
                state.logs.info(
                    "device",
                    format!(
                        "← {}",
                        serde_json::to_string(&reply).unwrap_or_else(|_| "<unprintable>".into())
                    ),
                );
                return Ok(result);
            }
            TickEvent::Ble(Ok(crate::transport::ble::BleEvent::Reply(reply))) => {
                let result = result_from_reply(reply.clone());
                state.logs.info(
                    "device",
                    format!(
                        "← {}",
                        serde_json::to_string(&reply).unwrap_or_else(|_| "<unprintable>".into())
                    ),
                );
                return Ok(result);
            }
            TickEvent::Usb(Ok(crate::transport::usb::UsbEvent::Log(line)))
            | TickEvent::Ble(Ok(crate::transport::ble::BleEvent::Log(line))) => {
                state.logs.info("device-log", line);
            }
            TickEvent::Usb(Ok(crate::transport::usb::UsbEvent::Disconnected(reason)))
            | TickEvent::Ble(Ok(crate::transport::ble::BleEvent::Disconnected(reason))) => {
                clear_link(state);
                return Err(AppError::device_disconnected(reason));
            }
            TickEvent::Usb(Err(_)) | TickEvent::Ble(Err(_)) => continue,
        }
    }
}

enum TickEvent {
    Usb(Result<crate::transport::usb::UsbEvent, mpsc::RecvTimeoutError>),
    Ble(Result<crate::transport::ble::BleEvent, mpsc::RecvTimeoutError>),
}

fn clear_link(state: &AppState) {
    if let Ok(mut g) = state.link.lock() {
        *g = LinkState::Disconnected;
    }
}

#[tauri::command]
pub async fn list_usb_ports() -> Result<Vec<String>, AppError> {
    Ok(tauri::async_runtime::spawn_blocking(crate::transport::usb::list_ports)
        .await
        .map_err(|e| AppError::internal(format!("list_usb_ports task: {e}")))?)
}

#[tauri::command]
pub async fn connect_usb(
    port: String,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    let port_for_log = port.clone();
    let link = tauri::async_runtime::spawn_blocking(move || UsbLink::connect(&port))
        .await
        .map_err(|e| AppError::internal(format!("connect task: {e}")))?
        .map_err(|e| AppError::usb_open_failed(e.to_string()))?;
    let shared = state.inner().clone();
    {
        let mut g = shared
            .link
            .lock()
            .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
        *g = LinkState::Usb(UsbHandle::new(link));
    }
    shared
        .logs
        .info("device", format!("USB connected · {port_for_log}"));
    emit_connection_changed(&shared);
    Ok(())
}

#[tauri::command]
pub async fn discover_ble() -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(BleLink::discover)
        .await
        .map_err(|e| AppError::internal(format!("discover task: {e}")))?
        .map_err(|e| AppError::ble_connect_failed(e.to_string()))
}

#[tauri::command]
pub async fn connect_ble(state: State<'_, SharedState>) -> Result<(), AppError> {
    let link = tauri::async_runtime::spawn_blocking(BleLink::connect)
        .await
        .map_err(|e| AppError::internal(format!("BLE connect task: {e}")))?
        .map_err(|e| AppError::ble_connect_failed(e.to_string()))?;
    let shared = state.inner().clone();
    {
        let mut g = shared
            .link
            .lock()
            .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
        *g = LinkState::Ble(BleHandle::new(link));
    }
    shared.logs.info("device", "BLE connected · Inkpaper");
    emit_connection_changed(&shared);
    Ok(())
}

#[tauri::command]
pub async fn disconnect_device(state: State<'_, SharedState>) -> Result<(), AppError> {
    let shared = state.inner().clone();
    {
        let mut g = shared
            .link
            .lock()
            .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
        *g = LinkState::Disconnected;
    }
    shared.logs.info("device", "Device disconnected");
    emit_connection_changed(&shared);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStateInfo {
    pub connected: bool,
    pub kind: String,
    pub port: String,
}

#[tauri::command]
pub fn get_connection_state(state: State<'_, SharedState>) -> Result<ConnectionStateInfo, AppError> {
    let shared = state.inner();
    let g = shared
        .link
        .lock()
        .map_err(|e| AppError::internal(format!("link mutex poisoned: {e}")))?;
    Ok(ConnectionStateInfo {
        connected: g.is_connected(),
        kind: g.kind_label().into(),
        port: g.port_label(),
    })
}

#[tauri::command]
pub async fn get_device_status(
    state: State<'_, SharedState>,
) -> Result<DeviceCommandResult, AppError> {
    let shared = state.inner().clone();
    emit_sync_started(&shared, "status");
    let res = tauri::async_runtime::spawn_blocking(move || {
        send_and_wait(&shared, Command::GetStatus)
    })
    .await
    .map_err(|e| AppError::internal(format!("status task: {e}")))?;
    match &res {
        Ok(_) => emit_sync_finished(&state, "status", true, None),
        Err(e) => emit_sync_finished(&state, "status", false, Some(e.message.clone())),
    }
    res
}

#[tauri::command]
pub async fn set_wifi(
    ssid: String,
    password: String,
    state: State<'_, SharedState>,
) -> Result<DeviceCommandResult, AppError> {
    let ssid = ssid.trim().to_string();
    if ssid.is_empty() {
        return Err(AppError::invalid_input("SSID", "must not be empty"));
    }
    let shared = state.inner().clone();
    shared
        .logs
        .info("device", format!("set_wifi: ssid={ssid} (password redacted)"));
    let cmd = Command::SetWifi { ssid, password };
    tauri::async_runtime::spawn_blocking(move || send_and_wait(&shared, cmd))
        .await
        .map_err(|e| AppError::internal(format!("set_wifi task: {e}")))?
}

#[tauri::command]
pub async fn set_server(
    url: String,
    token: String,
    state: State<'_, SharedState>,
) -> Result<DeviceCommandResult, AppError> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err(AppError::invalid_input("URL", "must not be empty"));
    }
    let shared = state.inner().clone();
    shared.logs.info(
        "device",
        format!(
            "set_server: url={url} (token redacted: {})",
            crate::commands::logs::redact_secret(&token)
        ),
    );
    let cmd = Command::SetServer { url, token };
    tauri::async_runtime::spawn_blocking(move || send_and_wait(&shared, cmd))
        .await
        .map_err(|e| AppError::internal(format!("set_server task: {e}")))?
}

#[tauri::command]
pub async fn set_timezone(
    offset_minutes: i16,
    state: State<'_, SharedState>,
) -> Result<DeviceCommandResult, AppError> {
    if !(-14 * 60..=14 * 60).contains(&offset_minutes) || offset_minutes % 15 != 0 {
        return Err(AppError::invalid_input(
            "offset_minutes",
            "must be a multiple of 15 between -14:00 and +14:00",
        ));
    }
    let shared = state.inner().clone();
    shared.logs.info(
        "device",
        format!("set_timezone: offset_minutes={offset_minutes}"),
    );
    let cmd = Command::SetTimezone { offset_minutes };
    tauri::async_runtime::spawn_blocking(move || send_and_wait(&shared, cmd))
        .await
        .map_err(|e| AppError::internal(format!("set_timezone task: {e}")))?
}

#[tauri::command]
pub async fn sync_now(state: State<'_, SharedState>) -> Result<DeviceCommandResult, AppError> {
    let shared = state.inner().clone();
    emit_sync_started(&shared, "sync");
    let res = tauri::async_runtime::spawn_blocking(move || {
        send_and_wait(&shared, Command::SyncNow)
    })
    .await
    .map_err(|e| AppError::internal(format!("sync task: {e}")))?;
    match &res {
        Ok(_) => emit_sync_finished(&state, "sync", true, None),
        Err(e) => emit_sync_finished(&state, "sync", false, Some(e.message.clone())),
    }
    res
}

#[tauri::command]
pub async fn clear_device_alarms(
    state: State<'_, SharedState>,
) -> Result<DeviceCommandResult, AppError> {
    let shared = state.inner().clone();
    shared.logs.warn("device", "clear_device_alarms");
    tauri::async_runtime::spawn_blocking(move || send_and_wait(&shared, Command::ClearAlarms))
        .await
        .map_err(|e| AppError::internal(format!("clear_alarms task: {e}")))?
}

fn emit_connection_changed(state: &Arc<AppState>) {
    let info = {
        let g = match state.link.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        ConnectionStateInfo {
            connected: g.is_connected(),
            kind: g.kind_label().into(),
            port: g.port_label(),
        }
    };
    let _ = state.ctx.emit("connection-changed", &info);
}

fn emit_sync_started(state: &Arc<AppState>, action: &str) {
    let _ = state
        .ctx
        .emit("sync-started", serde_json::json!({ "action": action }));
}

fn emit_sync_finished(state: &State<'_, SharedState>, action: &str, ok: bool, error: Option<String>) {
    let _ = state.ctx.emit(
        "sync-finished",
        serde_json::json!({ "action": action, "ok": ok, "error": error }),
    );
}

#[allow(dead_code)]
fn _unused_app_handle(_h: &AppHandle) {}
