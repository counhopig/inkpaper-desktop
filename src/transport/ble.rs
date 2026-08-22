//! BLE transport, matching `inkwash/docs/control-protocol.md`'s BLE
//! Framing section: commands are written to the write characteristic as
//! plain JSON (no line framing needed - GATT writes are already
//! message-delimited), replies arrive as notifications on the separate
//! notify characteristic. Runs its own Tokio runtime on a dedicated
//! thread, since `btleplug` is async-only and the Tauri runtime's
//! tokio-blocking boundary cannot host it directly - same "worker
//! thread + `std::sync::mpsc` channel" shape as `transport::usb`, just
//! with an async worker body instead of a blocking one.

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
const DEVICE_NAME: &str = "Inkwash";

pub enum BleEvent {
    /// `id` is the reply's correlation id (see `protocol::decode_reply`),
    /// `None` if the device didn't echo one back.
    Reply(Option<String>, Reply),
    Log(String),
    Disconnected(String),
}

pub struct BleLink {
    pub(crate) cmd_tx: mpsc::Sender<(String, Command)>,
    pub event_rx: mpsc::Receiver<BleEvent>,
}

impl BleLink {
    /// Diagnostic snapshot of every peripheral CoreBluetooth exposed during
    /// a short scan. Used by the CLI to distinguish filtering bugs from a
    /// scan that receives no advertisements at all.
    pub fn scan_report() -> anyhow::Result<Vec<String>> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let manager = Manager::new().await?;
            let adapter = manager
                .adapters()
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("no BLE adapter available"))?;
            adapter.start_scan(ScanFilter::default()).await?;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let mut report = Vec::new();
            for peripheral in adapter.peripherals().await? {
                match peripheral.properties().await {
                    Ok(Some(props)) => report.push(format!("{props:?}")),
                    Ok(None) => report.push(format!("{:?}: no properties", peripheral.id())),
                    Err(err) => report.push(format!("{:?}: {err}", peripheral.id())),
                }
            }
            adapter.stop_scan().await.ok();
            Ok(report)
        })
    }

    /// Background-friendly discovery probe. The firmware advertises only
    /// while its BLE Pairing page is open.
    pub fn discover() -> anyhow::Result<bool> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let manager = Manager::new().await?;
            let adapters = manager.adapters().await?;
            let Some(adapter) = adapters.into_iter().next() else {
                return Err(anyhow::anyhow!(
                    "no BLE adapter is available to this app; check that Bluetooth is on and allow Bluetooth access in System Settings > Privacy & Security > Bluetooth"
                ));
            };
            adapter.start_scan(ScanFilter::default()).await?;
            let found = find_device_with_retries(&adapter, DEVICE_NAME, 8)
                .await
                .is_ok();
            adapter.stop_scan().await.ok();
            Ok(found)
        })
    }

    /// Scans for a device named `DEVICE_NAME`, connects, and subscribes to
    /// the reply characteristic. Blocks the calling thread until connected
    /// or the scan/connect attempt fails - call this from a worker thread
    /// or `tokio::task::spawn_blocking`, not directly from the UI's
    /// `update()`.
    pub fn connect() -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<(String, Command)>();
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
                match connect_and_run(cmd_rx, event_tx.clone(), &ready_tx).await {
                    Ok(()) => {}
                    Err(e) => {
                        // If setup failed this wakes connect(); if setup had
                        // already succeeded, the receiver is gone and the
                        // disconnect event below is what the UI observes.
                        let _ = ready_tx.send(Err(anyhow::anyhow!("{e}")));
                        let _ = event_tx.send(BleEvent::Disconnected(e.to_string()));
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

    /// `id` is the request correlation id to attach - generate one with
    /// `protocol::next_request_id()` and reuse it across resends of the
    /// same logical request.
    #[allow(dead_code)]
    pub fn send(&self, id: &str, cmd: Command) -> anyhow::Result<()> {
        self.cmd_tx
            .send((id.to_string(), cmd))
            .map_err(|_| anyhow::anyhow!("BLE worker thread is gone"))
    }
}

async fn connect_and_run(
    cmd_rx: mpsc::Receiver<(String, Command)>,
    event_tx: mpsc::Sender<BleEvent>,
    ready_tx: &mpsc::Sender<anyhow::Result<()>>,
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

    // Report readiness as soon as GATT setup is complete. Previously this
    // was sent only after this long-running loop returned, so the Desktop
    // UI could never receive a live BleLink.
    ready_tx
        .send(Ok(()))
        .map_err(|_| anyhow::anyhow!("BLE connection requester went away"))?;

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
        while let Ok((id, cmd)) = cmd_rx.try_recv() {
            let payload = protocol::encode_command(&cmd, &id);
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
                    Ok((id, reply)) => {
                        if event_tx.send(BleEvent::Reply(id, reply)).is_err() {
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
    find_device_with_retries(adapter, name, 20).await
}

async fn find_device_with_retries(
    adapter: &btleplug::platform::Adapter,
    name: &str,
    retries: usize,
) -> anyhow::Result<Peripheral> {
    let service_uuid = Uuid::parse_str(SERVICE_UUID)?;
    // A handful of short retries rather than one long wait: advertising
    // packets aren't guaranteed to be seen on the very first scan pass,
    // but this device only advertises while its BLE Pairing screen is
    // open (see `docs/control-protocol.md`'s Lifecycle notes), so we
    // don't want to hang indefinitely if the user hasn't opened it yet.
    for _ in 0..retries {
        for p in adapter.peripherals().await? {
            if let Ok(Some(props)) = p.properties().await {
                // CoreBluetooth does not consistently expose local_name for
                // unpaired peripherals. The advertised service UUID is the
                // stable identity and is also more specific than the name.
                if props.local_name.as_deref() == Some(name)
                    || props.services.contains(&service_uuid)
                {
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
