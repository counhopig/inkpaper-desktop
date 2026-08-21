//! USB serial transport, matching `inkwash/docs/control-protocol.md`'s
//! framing: commands go out as `>>IW {json}\n`, replies come back as
//! `<<IW {json}\n` on the same line-oriented stream that also carries the
//! device's ordinary `log::info!` output - any line without the `<<IW `
//! prefix is just log noise from this reader's point of view and is
//! surfaced as a `Log` event instead of discarded, so the UI can show it
//! for debugging.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::protocol::{self, Command, Reply};

const COMMAND_PREFIX: &str = ">>IW ";
const REPLY_PREFIX: &str = "<<IW ";
const BAUD_RATE: u32 = 115_200;

pub enum UsbEvent {
    Reply(Reply),
    Log(String),
    /// The port closed or errored; the worker thread has exited.
    Disconnected(String),
}

pub struct UsbLink {
    pub(crate) cmd_tx: mpsc::Sender<Command>,
    pub event_rx: mpsc::Receiver<UsbEvent>,
}

impl UsbLink {
    /// Opens `port_name` and spawns the reader/writer worker thread.
    pub fn connect(port_name: &str) -> anyhow::Result<Self> {
        let mut port = serialport::new(port_name, BAUD_RATE)
            .timeout(Duration::from_millis(200))
            .open()
            .map_err(|e| anyhow::anyhow!("failed to open {port_name}: {e}"))?;

        // ESP32-S3 USB Serial/JTAG uses modem-control lines for reset and
        // download-mode entry. Explicitly release both after opening; host
        // defaults vary and can otherwise leave the device silent while the
        // port itself still appears to have opened successfully.
        port.write_data_terminal_ready(false)
            .map_err(|e| anyhow::anyhow!("failed to release DTR on {port_name}: {e}"))?;
        port.write_request_to_send(false)
            .map_err(|e| anyhow::anyhow!("failed to release RTS on {port_name}: {e}"))?;

        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<UsbEvent>();

        thread::spawn(move || run_worker(port, cmd_rx, event_tx));

        Ok(Self { cmd_tx, event_rx })
    }

    pub fn send(&self, cmd: Command) -> anyhow::Result<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| anyhow::anyhow!("USB worker thread is gone"))
    }
}

fn run_worker(
    mut port: Box<dyn serialport::SerialPort>,
    cmd_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<UsbEvent>,
) {
    let mut line_buf: Vec<u8> = Vec::new();
    let mut read_buf = [0u8; 256];
    loop {
        // Drain any queued outgoing commands first - cheap, and keeps
        // command latency low relative to the read timeout below.
        while let Ok(cmd) = cmd_rx.try_recv() {
            let line = format!("{COMMAND_PREFIX}{}\n", protocol::encode_command(&cmd));
            if let Err(e) = port.write_all(line.as_bytes()) {
                let _ = event_tx.send(UsbEvent::Disconnected(format!("write failed: {e}")));
                return;
            }
        }

        match port.read(&mut read_buf) {
            Ok(0) => {}
            Ok(n) => {
                for &b in &read_buf[..n] {
                    if b == b'\n' {
                        let line = String::from_utf8_lossy(&line_buf)
                            .trim_end_matches('\r')
                            .to_string();
                        line_buf.clear();
                        if let Some(json) = line.strip_prefix(REPLY_PREFIX) {
                            match protocol::decode_reply(json) {
                                Ok(reply) => {
                                    if event_tx.send(UsbEvent::Reply(reply)).is_err() {
                                        return;
                                    }
                                }
                                Err(err) => {
                                    let _ = event_tx
                                        .send(UsbEvent::Log(format!("(unparseable reply: {err})")));
                                }
                            }
                        } else if !line.is_empty() {
                            let _ = event_tx.send(UsbEvent::Log(line));
                        }
                    } else {
                        line_buf.push(b);
                    }
                }
            }
            // A read timeout with nothing available is the normal "no data
            // yet" case for a port opened with a fixed timeout, not an
            // error - only genuine I/O errors below should end the worker.
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                let _ = event_tx.send(UsbEvent::Disconnected(format!("read failed: {e}")));
                return;
            }
        }
    }
}

/// Lists available serial port names for the connection picker.
pub fn list_ports() -> Vec<String> {
    let ports = serialport::available_ports().unwrap_or_default();
    let mut espressif: Vec<String> = ports
        .iter()
        .filter(|port| {
            matches!(
                &port.port_type,
                serialport::SerialPortType::UsbPort(info) if info.vid == 0x303a
            )
        })
        .map(|port| port.port_name.clone())
        .collect();
    if espressif.is_empty() {
        espressif = ports
            .into_iter()
            .filter(|port| matches!(port.port_type, serialport::SerialPortType::UsbPort(_)))
            .map(|port| port.port_name)
            .collect();
    }
    espressif.sort();
    espressif
}
