//! Tauri command surface, grouped by responsibility:
//!
//! * `device` - USB/BLE connection lifecycle, sending commands to the
//!   physical Inkpaper, polling its status.
//! * `server` - admin HTTP API for `inkpaper-server` (devices, alarms,
//!   todos). All of these run on `spawn_blocking` since the underlying
//!   `reqwest` client is blocking.
//! * `logs_cmd` - log entry read/clear and log-file path discovery.

pub mod device;
pub mod logs;
pub mod logs_cmd;
pub mod server;
