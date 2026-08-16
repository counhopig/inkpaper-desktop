//! BLE transport, matching `inkpaper/docs/control-protocol.md`'s BLE
//! Framing section: commands are written to the write characteristic as
//! plain JSON (no line framing needed - GATT writes are already
//! message-delimited), replies arrive as notifications on the separate
//! notify characteristic. Runs its own Tokio runtime on a dedicated
//! thread, since `btleplug` is async-only and the egui UI thread is not -
//! same "worker thread + `std::sync::mpsc` channel" shape as
//! `transport::usb`, just with an async worker body instead of a blocking
//! one.

use std::sync::mpsc;
use std::thread;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::protocol::{self, Command, Reply};

const SERVICE_UUID: &str = "d2c25e50-5e22-48d8-a8b3-34f2f8e2c7d4";
const WRITE_CHAR_UUID: &str = "d2c25e51-5e22-48d8-a8b3-34f2f8e2c7d4";
const NOTIFY_CHAR_UUID: &str = "d2c25e52-5e22-48d8-a8b3-34f2f8e2c7d4";
/// Advertised device name set in `ble_control.rs::BleControl::start`.
const DEVICE_NAME: &str = "Inkpaper";

pub enum BleEvent {
    Reply(Reply),
    Log(String),
    Disconnected(String),
}

pub struct BleLink {
    cmd_tx: mpsc::Sender<Command>,
    pub event_rx: mpsc::Receiver<BleEvent>,
}

impl BleLink {
    /// Scans for a device named `DEVICE_NAME`, connects, and subscribes to
    /// the reply characteristic. Blocks the calling thread until connected
    /// or the scan/connect attempt fails - call this from a worker thread
    /// or `tokio::task::spawn_blocking`, not directly from the UI's
    /// `update()`.
    pub fn connect() -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<BleEvent>();
        let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<()>>();

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready_tx.send(Err(anyhow::anyhow!("tokio runtime init failed: {e}")));
                    return;
                }
            };
            rt.block_on(async {
                match connect_and_run(cmd_rx, event_tx.clone()).await {
                    Ok(()) => {
                        let _ = ready_tx.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(anyhow::anyhow!("{e}")));
                    }
                }
            });
        });

        // The worker signals readiness only after it has actually
        // connected and subscribed (see `connect_and_run`'s first phase),
        // so a caller blocking on this knows the link is live before it
        // returns, not just that the thread started.
        ready_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("BLE worker thread exited before connecting"))??;

        Ok(Self { cmd_tx, event_rx })
    }

    pub fn send(&self, cmd: Command) -> anyhow::Result<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| anyhow::anyhow!("BLE worker thread is gone"))
    }
}

async fn connect_and_run(
    cmd_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<BleEvent>,
) -> anyhow::Result<()> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no BLE adapter found"))?;

    adapter.start_scan(ScanFilter::default()).await?;
    let peripheral = find_device(&adapter, DEVICE_NAME).await?;
    adapter.stop_scan().await.ok();

    peripheral.connect().await?;
    peripheral.discover_services().await?;

    let write_uuid = Uuid::parse_str(WRITE_CHAR_UUID)?;
    let notify_uuid = Uuid::parse_str(NOTIFY_CHAR_UUID)?;
    let chars = peripheral.characteristics();
    let write_char = chars
        .iter()
        .find(|c| c.uuid == write_uuid)
        .ok_or_else(|| anyhow::anyhow!("write characteristic not found"))?
        .clone();
    let notify_char = chars
        .iter()
        .find(|c| c.uuid == notify_uuid)
        .ok_or_else(|| anyhow::anyhow!("notify characteristic not found"))?
        .clone();

    peripheral.subscribe(&notify_char).await?;
    let mut notifications = peripheral.notifications().await?;

    let _ = event_tx.send(BleEvent::Log(format!(
        "connected to {DEVICE_NAME} (service {SERVICE_UUID})"
    )));

    // From here on this task owns both directions: draining outgoing
    // commands and forwarding incoming notifications. `cmd_rx` is a
    // blocking `std::sync::mpsc::Receiver`, so it's polled via
    // `try_recv()` inside the same `select!`-free loop rather than
    // `.await`ed directly - a short sleep keeps this from busy-spinning
    // between notification arrivals.
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            let payload = protocol::encode_command(&cmd);
            if let Err(e) = peripheral
                .write(&write_char, payload.as_bytes(), WriteType::WithResponse)
                .await
            {
                let _ = event_tx.send(BleEvent::Disconnected(format!("write failed: {e}")));
                return Ok(());
            }
        }

        match tokio::time::timeout(std::time::Duration::from_millis(200), notifications.next())
            .await
        {
            Ok(Some(data)) => {
                let text = String::from_utf8_lossy(&data.value).to_string();
                match protocol::decode_reply(&text) {
                    Ok(reply) => {
                        if event_tx.send(BleEvent::Reply(reply)).is_err() {
                            return Ok(());
                        }
                    }
                    Err(err) => {
                        let _ = event_tx.send(BleEvent::Log(format!("(unparseable reply: {err})")));
                    }
                }
            }
            Ok(None) => {
                let _ = event_tx.send(BleEvent::Disconnected("notification stream ended".into()));
                return Ok(());
            }
            Err(_timeout) => {} // no notification this tick, loop back to check cmd_rx
        }
    }
}

async fn find_device(
    adapter: &btleplug::platform::Adapter,
    name: &str,
) -> anyhow::Result<Peripheral> {
    // A handful of short retries rather than one long wait: advertising
    // packets aren't guaranteed to be seen on the very first scan pass,
    // but this device only advertises while its BLE Pairing screen is
    // open (see `docs/control-protocol.md`'s Lifecycle notes), so we
    // don't want to hang indefinitely if the user hasn't opened it yet.
    for _ in 0..20 {
        for p in adapter.peripherals().await? {
            if let Ok(Some(props)) = p.properties().await {
                if props.local_name.as_deref() == Some(name) {
                    return Ok(p);
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Err(anyhow::anyhow!(
        "no BLE device named '{name}' found after scanning - make sure the device's BLE Pairing screen is open"
    ))
}
