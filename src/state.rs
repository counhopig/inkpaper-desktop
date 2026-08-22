//! Shared application state held by the Tauri runtime.
//!
//! Two invariants to be aware of when reading this file:
//!
//! 1. `link` is *never* held across an `event_rx.recv_timeout()` - the
//!    wait happens against the per-link receiver mutex instead, so a
//!    slow USB sync does not block BLE scans, log reads, or any other
//!    command that needs to look at the link state.
//! 2. `logs.append()` always logs to stderr AND to the on-disk file AND
//!    emits a `device-log` event - CLI mode picks up the stderr line,
//!    the GUI picks up the event, the file is the permanent record.

use std::sync::{mpsc, Mutex};
use std::time::Duration;

use tauri::AppHandle;

use crate::commands::logs::LogStore;
use crate::protocol::Command;
use crate::transport::{
    ble::{BleEvent, BleLink},
    usb::{UsbEvent, UsbLink},
};

pub struct AppState {
    pub link: Mutex<LinkState>,
    pub logs: LogStore,
    pub ctx: AppHandle,
}

impl AppState {
    pub fn new(ctx: AppHandle) -> Self {
        let logs = LogStore::new(ctx.clone());
        logs.info("app", "Inkwash Desktop started");
        Self {
            link: Mutex::new(LinkState::Disconnected),
            logs,
            ctx,
        }
    }
}

#[derive(Default)]
pub enum LinkState {
    #[default]
    Disconnected,
    Usb(UsbHandle),
    Ble(BleHandle),
}

impl LinkState {
    pub fn is_connected(&self) -> bool {
        !matches!(self, Self::Disconnected)
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Disconnected => "Offline",
            Self::Usb(_) => "USB",
            Self::Ble(_) => "BLE",
        }
    }

    pub fn port_label(&self) -> String {
        match self {
            Self::Disconnected => "—".into(),
            Self::Usb(_) => "USB serial".into(),
            Self::Ble(_) => "Inkwash (BLE)".into(),
        }
    }
}

/// Wraps a `UsbLink`'s command sender and receiver. The receiver lives
/// behind its own `Mutex` so the long blocking `recv_timeout()` in a
/// command does not serialise behind the application-wide `link` lock.
pub struct UsbHandle {
    cmd_tx: mpsc::Sender<(String, Command)>,
    event_rx: Mutex<mpsc::Receiver<UsbEvent>>,
}

impl UsbHandle {
    pub fn new(link: UsbLink) -> Self {
        let UsbLink { cmd_tx, event_rx } = link;
        Self {
            cmd_tx,
            event_rx: Mutex::new(event_rx),
        }
    }

    /// `id` is the request correlation id to attach - generate one with
    /// `protocol::next_request_id()` and reuse it across resends of the
    /// same logical request.
    pub fn send(&self, id: &str, cmd: Command) -> Result<(), crate::error::AppError> {
        self.cmd_tx
            .send((id.to_string(), cmd))
            .map_err(|_| crate::error::AppError::internal("USB worker thread is gone"))
    }

    pub fn recv_timeout(&self, dur: Duration) -> Result<UsbEvent, mpsc::RecvTimeoutError> {
        self.event_rx
            .lock()
            .expect("usb event_rx mutex poisoned")
            .recv_timeout(dur)
    }
}

/// Same shape as `UsbHandle`, for the BLE transport.
pub struct BleHandle {
    cmd_tx: mpsc::Sender<(String, Command)>,
    event_rx: Mutex<mpsc::Receiver<BleEvent>>,
}

impl BleHandle {
    pub fn new(link: BleLink) -> Self {
        let BleLink { cmd_tx, event_rx } = link;
        Self {
            cmd_tx,
            event_rx: Mutex::new(event_rx),
        }
    }

    /// `id` is the request correlation id to attach - generate one with
    /// `protocol::next_request_id()` and reuse it across resends of the
    /// same logical request.
    pub fn send(&self, id: &str, cmd: Command) -> Result<(), crate::error::AppError> {
        self.cmd_tx
            .send((id.to_string(), cmd))
            .map_err(|_| crate::error::AppError::internal("BLE worker thread is gone"))
    }

    pub fn recv_timeout(&self, dur: Duration) -> Result<BleEvent, mpsc::RecvTimeoutError> {
        self.event_rx
            .lock()
            .expect("ble event_rx mutex poisoned")
            .recv_timeout(dur)
    }
}
