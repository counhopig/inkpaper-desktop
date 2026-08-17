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
    ClearAlarms,
    SetTimezone { offset_minutes: i16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reply {
    Ok,
    Status {
        wifi_configured: bool,
        server_configured: bool,
        wifi_connected: bool,
        #[serde(default)]
        wifi_ssid: Option<String>,
        #[serde(default)]
        wifi_has_password: bool,
        #[serde(default)]
        server_url: Option<String>,
        #[serde(default)]
        server_has_token: bool,
        #[serde(default)]
        timezone_offset_minutes: i16,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_commands_use_expected_wire_names() {
        assert_eq!(
            encode_command(&Command::GetStatus),
            r#"{"cmd":"get_status"}"#
        );
        assert_eq!(
            encode_command(&Command::ClearAlarms),
            r#"{"cmd":"clear_alarms"}"#
        );
        assert_eq!(
            encode_command(&Command::SetTimezone {
                offset_minutes: 480
            }),
            r#"{"cmd":"set_timezone","offset_minutes":480}"#
        );
    }

    #[test]
    fn decodes_status_reply() {
        let reply = decode_reply(
            r#"{"status":"status","wifi_configured":true,"server_configured":false,"wifi_connected":true,"wifi_ssid":"MyNet","wifi_has_password":true,"server_url":"http://192.168.1.10:8080/api/sync","server_has_token":true,"timezone_offset_minutes":480}"#,
        )
        .unwrap();
        match reply {
            Reply::Status {
                wifi_configured,
                server_configured,
                wifi_connected,
                wifi_ssid,
                wifi_has_password,
                server_url,
                server_has_token,
                timezone_offset_minutes,
            } => {
                assert!(wifi_configured);
                assert!(!server_configured);
                assert!(wifi_connected);
                assert_eq!(wifi_ssid.as_deref(), Some("MyNet"));
                assert!(wifi_has_password);
                assert_eq!(
                    server_url.as_deref(),
                    Some("http://192.168.1.10:8080/api/sync")
                );
                assert!(server_has_token);
                assert_eq!(timezone_offset_minutes, 480);
            }
            _ => panic!("expected Status reply"),
        }
    }

    #[test]
    fn decodes_legacy_status_reply_without_new_fields() {
        let reply = decode_reply(
            r#"{"status":"status","wifi_configured":true,"server_configured":false,"wifi_connected":false}"#,
        )
        .unwrap();
        match reply {
            Reply::Status {
                wifi_ssid,
                wifi_has_password,
                server_url,
                server_has_token,
                timezone_offset_minutes,
                ..
            } => {
                assert!(wifi_ssid.is_none());
                assert!(!wifi_has_password);
                assert!(server_url.is_none());
                assert!(!server_has_token);
                assert_eq!(timezone_offset_minutes, 0);
            }
            _ => panic!("expected Status reply"),
        }
    }
}
