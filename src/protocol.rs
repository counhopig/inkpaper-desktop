//! Wire types mirroring the firmware's `control.rs`
//! (`inkpaper/docs/control-protocol.md`) - this crate sends `Command` JSON
//! and parses `Reply` JSON, so the tag/field names must match the
//! firmware's `Serialize`/`Deserialize` derives exactly. USB and BLE both
//! carry the same JSON payloads; only the framing differs (see
//! `transport::usb` vs `transport::ble`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    SetWifi { ssid: String, password: String },
    SetServer { url: String, token: String },
    SyncNow,
    GetStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    Ok,
    Status {
        wifi_configured: bool,
        server_configured: bool,
        wifi_connected: bool,
    },
    Error {
        message: String,
    },
}

pub fn encode_command(cmd: &Command) -> String {
    serde_json::to_string(cmd).expect("Command always serializes")
}

pub fn decode_reply(line: &str) -> anyhow::Result<Reply> {
    serde_json::from_str(line).map_err(|e| anyhow::anyhow!("failed to parse reply '{line}': {e}"))
}
