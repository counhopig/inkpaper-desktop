//! egui/eframe UI. Two tabs sharing one window:
//! - **Device**: talk to the physical device over USB or BLE (Wi-Fi setup,
//!   server config, sync trigger, status) - `docs/control-protocol.md`.
//! - **Server**: manage a device's alarms/todos on `inkpaper-server`
//!   (`docs/sync-api.md` is what the *device* reads; this tab is the admin
//!   side that writes what ends up there).
//!
//! Every USB/BLE/HTTP operation runs on a background thread and reports
//! back through a channel the UI drains each frame - `update()` must never
//! block, or the whole window freezes for the duration of a serial read or
//! network request.

use std::sync::mpsc;
use std::thread;

use crate::protocol::{Command, Reply};
use crate::server::{self, ServerClient};
use crate::transport::{ble::BleLink, usb::UsbLink};

enum Tab {
    Device,
    Server,
}

enum DeviceLink {
    None,
    Usb(UsbLink),
    Ble(BleLink),
}

enum ServerEvent {
    Devices(Vec<server::Device>),
    DeviceRegistered(server::Device),
    Alarms(i64, Vec<server::Alarm>),
    Todos(i64, Vec<server::Todo>),
    ActionDone(String),
    Error(String),
}

pub struct App {
    tab: Tab,

    // --- Device tab state ---
    usb_ports: Vec<String>,
    selected_port: String,
    link: DeviceLink,
    wifi_ssid: String,
    wifi_password: String,
    device_server_url: String,
    device_server_token: String,
    device_log: Vec<String>,
    last_status: Option<(bool, bool, bool)>,

    // --- Server tab state ---
    server_base_url: String,
    server_admin_token: String,
    server_client: Option<ServerClient>,
    server_event_tx: mpsc::Sender<ServerEvent>,
    server_event_rx: mpsc::Receiver<ServerEvent>,
    devices: Vec<server::Device>,
    new_device_name: String,
    selected_device: Option<i64>,
    alarms: Vec<server::Alarm>,
    todos: Vec<server::Todo>,
    new_alarm_hour: u8,
    new_alarm_minute: u8,
    new_alarm_label: String,
    new_todo_text: String,
    server_status: String,
}

impl Default for App {
    fn default() -> Self {
        let (server_event_tx, server_event_rx) = mpsc::channel();
        Self {
            tab: Tab::Device,
            usb_ports: crate::transport::usb::list_ports(),
            selected_port: String::new(),
            link: DeviceLink::None,
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            device_server_url: String::new(),
            device_server_token: String::new(),
            device_log: Vec::new(),
            last_status: None,
            server_base_url: "http://127.0.0.1:8080".to_string(),
            server_admin_token: String::new(),
            server_client: None,
            server_event_tx,
            server_event_rx,
            devices: Vec::new(),
            new_device_name: String::new(),
            selected_device: None,
            alarms: Vec::new(),
            todos: Vec::new(),
            new_alarm_hour: 7,
            new_alarm_minute: 0,
            new_alarm_label: String::new(),
            new_todo_text: String::new(),
            server_status: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_device_events();
        self.drain_server_events();

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Device, "Device (USB/BLE)");
                ui.selectable_value(&mut self.tab, Tab::Server, "Server (alarms/todos)");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Device => self.device_tab(ui),
            Tab::Server => self.server_tab(ui),
        });

        // Background threads deliver results asynchronously; polling
        // roughly every 100ms is simpler than plumbing `ctx.clone()` into
        // every worker thread just to call `request_repaint()` precisely,
        // and imperceptible for a config tool that isn't animating.
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Tab::Device, Tab::Device) | (Tab::Server, Tab::Server)
        )
    }
}

impl App {
    fn drain_device_events(&mut self) {
        let events: Vec<_> = match &self.link {
            DeviceLink::Usb(link) => link.event_rx.try_iter().map(DeviceEvent::Usb).collect(),
            DeviceLink::Ble(link) => link.event_rx.try_iter().map(DeviceEvent::Ble).collect(),
            DeviceLink::None => Vec::new(),
        };
        for event in events {
            match event {
                DeviceEvent::Usb(crate::transport::usb::UsbEvent::Reply(reply))
                | DeviceEvent::Ble(crate::transport::ble::BleEvent::Reply(reply)) => {
                    self.handle_reply(reply);
                }
                DeviceEvent::Usb(crate::transport::usb::UsbEvent::Log(line))
                | DeviceEvent::Ble(crate::transport::ble::BleEvent::Log(line)) => {
                    self.device_log.push(line);
                }
                DeviceEvent::Usb(crate::transport::usb::UsbEvent::Disconnected(reason))
                | DeviceEvent::Ble(crate::transport::ble::BleEvent::Disconnected(reason)) => {
                    self.device_log.push(format!("disconnected: {reason}"));
                    self.link = DeviceLink::None;
                }
            }
        }
    }

    fn handle_reply(&mut self, reply: Reply) {
        match reply {
            Reply::Ok => self.device_log.push("OK".to_string()),
            Reply::Status {
                wifi_configured,
                server_configured,
                wifi_connected,
            } => {
                self.last_status = Some((wifi_configured, server_configured, wifi_connected));
                self.device_log.push(format!(
                    "status: wifi_configured={wifi_configured} server_configured={server_configured} wifi_connected={wifi_connected}"
                ));
            }
            Reply::Error { message } => self.device_log.push(format!("error: {message}")),
        }
    }

    fn send_command(&mut self, cmd: Command) {
        let result = match &self.link {
            DeviceLink::Usb(link) => link.send(cmd),
            DeviceLink::Ble(link) => link.send(cmd),
            DeviceLink::None => Err(anyhow::anyhow!("not connected")),
        };
        if let Err(err) = result {
            self.device_log.push(format!("send failed: {err}"));
        }
    }

    fn device_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connection");
        ui.horizontal(|ui| {
            if ui.button("Refresh USB ports").clicked() {
                self.usb_ports = crate::transport::usb::list_ports();
            }
            egui::ComboBox::from_label("Port")
                .selected_text(if self.selected_port.is_empty() {
                    "(select)".to_string()
                } else {
                    self.selected_port.clone()
                })
                .show_ui(ui, |ui| {
                    for port in &self.usb_ports {
                        ui.selectable_value(&mut self.selected_port, port.clone(), port);
                    }
                });
            if ui.button("Connect USB").clicked() && !self.selected_port.is_empty() {
                match UsbLink::connect(&self.selected_port) {
                    Ok(link) => {
                        self.link = DeviceLink::Usb(link);
                        self.device_log.push(format!("connected to {}", self.selected_port));
                    }
                    Err(err) => self.device_log.push(format!("connect failed: {err}")),
                }
            }
            if ui.button("Connect BLE").clicked() {
                self.device_log.push("scanning for BLE device 'Inkpaper'...".to_string());
                match BleLink::connect() {
                    Ok(link) => {
                        self.link = DeviceLink::Ble(link);
                        self.device_log.push("BLE connected".to_string());
                    }
                    Err(err) => self.device_log.push(format!("BLE connect failed: {err}")),
                }
            }
            let connected = !matches!(self.link, DeviceLink::None);
            ui.colored_label(
                if connected { egui::Color32::GREEN } else { egui::Color32::GRAY },
                if connected { "connected" } else { "not connected" },
            );
        });

        ui.separator();
        ui.heading("Wi-Fi");
        ui.horizontal(|ui| {
            ui.label("SSID");
            ui.text_edit_singleline(&mut self.wifi_ssid);
        });
        ui.horizontal(|ui| {
            ui.label("Password");
            ui.add(egui::TextEdit::singleline(&mut self.wifi_password).password(true));
        });
        if ui.button("Send Wi-Fi credentials").clicked() {
            self.send_command(Command::SetWifi {
                ssid: self.wifi_ssid.clone(),
                password: self.wifi_password.clone(),
            });
        }

        ui.separator();
        ui.heading("Server");
        ui.horizontal(|ui| {
            ui.label("Sync URL");
            ui.text_edit_singleline(&mut self.device_server_url);
        });
        ui.horizontal(|ui| {
            ui.label("Device token");
            ui.text_edit_singleline(&mut self.device_server_token);
        });
        ui.label("(paste the token from the Server tab's \"Register\" result here)");
        if ui.button("Send server config").clicked() {
            self.send_command(Command::SetServer {
                url: self.device_server_url.clone(),
                token: self.device_server_token.clone(),
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Sync Now").clicked() {
                self.send_command(Command::SyncNow);
            }
            if ui.button("Get Status").clicked() {
                self.send_command(Command::GetStatus);
            }
        });

        ui.separator();
        ui.heading("Log");
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for line in self.device_log.iter().rev().take(200) {
                ui.monospace(line);
            }
        });
    }

    fn drain_server_events(&mut self) {
        for event in self.server_event_rx.try_iter().collect::<Vec<_>>() {
            match event {
                ServerEvent::Devices(devices) => self.devices = devices,
                ServerEvent::DeviceRegistered(device) => {
                    self.server_status = format!(
                        "Registered '{}' - token: {}",
                        device.name,
                        device.token.as_deref().unwrap_or("?")
                    );
                    self.refresh_devices();
                }
                ServerEvent::Alarms(device_id, alarms) => {
                    if self.selected_device == Some(device_id) {
                        self.alarms = alarms;
                    }
                }
                ServerEvent::Todos(device_id, todos) => {
                    if self.selected_device == Some(device_id) {
                        self.todos = todos;
                    }
                }
                ServerEvent::ActionDone(msg) => {
                    self.server_status = msg;
                    if let Some(id) = self.selected_device {
                        self.refresh_alarms(id);
                        self.refresh_todos(id);
                    }
                }
                ServerEvent::Error(err) => self.server_status = format!("error: {err}"),
            }
        }
    }

    fn client(&mut self) -> Option<ServerClient> {
        if self.server_client.is_none() && !self.server_admin_token.is_empty() {
            self.server_client = Some(ServerClient::new(
                self.server_base_url.clone(),
                self.server_admin_token.clone(),
            ));
        }
        self.server_client.clone()
    }

    fn refresh_devices(&mut self) {
        let Some(client) = self.client() else { return };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.list_devices() {
                Ok(devices) => ServerEvent::Devices(devices),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn refresh_alarms(&mut self, device_id: i64) {
        let Some(client) = self.client() else { return };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.list_alarms(device_id) {
                Ok(alarms) => ServerEvent::Alarms(device_id, alarms),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn refresh_todos(&mut self, device_id: i64) {
        let Some(client) = self.client() else { return };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.list_todos(device_id) {
                Ok(todos) => ServerEvent::Todos(device_id, todos),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn server_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Server connection");
        ui.horizontal(|ui| {
            ui.label("Base URL");
            ui.text_edit_singleline(&mut self.server_base_url);
        });
        ui.horizontal(|ui| {
            ui.label("Admin token");
            if ui.add(egui::TextEdit::singleline(&mut self.server_admin_token).password(true)).changed() {
                self.server_client = None;
            }
        });
        if ui.button("Connect / Refresh devices").clicked() {
            self.server_client = None;
            self.refresh_devices();
        }
        ui.label(&self.server_status);

        ui.separator();
        ui.heading("Devices");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_device_name);
            if ui.button("Register new device").clicked() && !self.new_device_name.is_empty() {
                if let Some(client) = self.client() {
                    let tx = self.server_event_tx.clone();
                    let name = self.new_device_name.clone();
                    thread::spawn(move || {
                        let event = match client.register_device(&name) {
                            Ok(device) => ServerEvent::DeviceRegistered(device),
                            Err(err) => ServerEvent::Error(err.to_string()),
                        };
                        let _ = tx.send(event);
                    });
                }
            }
        });
        for device in self.devices.clone() {
            ui.horizontal(|ui| {
                let selected = self.selected_device == Some(device.id);
                if ui.selectable_label(selected, format!("#{} {}", device.id, device.name)).clicked() {
                    self.selected_device = Some(device.id);
                    self.refresh_alarms(device.id);
                    self.refresh_todos(device.id);
                }
                if ui.button("Delete").clicked() {
                    if let Some(client) = self.client() {
                        let tx = self.server_event_tx.clone();
                        thread::spawn(move || {
                            let event = match client.delete_device(device.id) {
                                Ok(()) => ServerEvent::ActionDone(format!("Deleted device #{}", device.id)),
                                Err(err) => ServerEvent::Error(err.to_string()),
                            };
                            let _ = tx.send(event);
                        });
                    }
                }
            });
        }

        let Some(device_id) = self.selected_device else {
            ui.label("Select a device above to manage its alarms/todos.");
            return;
        };

        ui.separator();
        ui.heading(format!("Alarms - device #{device_id}"));
        for alarm in self.alarms.clone() {
            ui.horizontal(|ui| {
                ui.label(format!("{:02}:{:02} {} [{}]", alarm.hour, alarm.minute, alarm.label, if alarm.enabled { "on" } else { "off" }));
                if ui.button(if alarm.enabled { "Disable" } else { "Enable" }).clicked() {
                    self.toggle_alarm(device_id, alarm.clone());
                }
                if ui.button("Delete").clicked() {
                    self.delete_alarm(device_id, alarm.id);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.new_alarm_hour).range(0..=23));
            ui.label(":");
            ui.add(egui::DragValue::new(&mut self.new_alarm_minute).range(0..=59));
            ui.text_edit_singleline(&mut self.new_alarm_label);
            if ui.button("Add daily alarm").clicked() {
                self.add_alarm(device_id);
            }
        });

        ui.separator();
        ui.heading(format!("Todos - device #{device_id}"));
        for todo in self.todos.clone() {
            ui.horizontal(|ui| {
                ui.label(format!("[{}] {}", if todo.done { "x" } else { " " }, todo.text));
                if ui.button(if todo.done { "Mark undone" } else { "Mark done" }).clicked() {
                    self.toggle_todo(device_id, todo.clone());
                }
                if ui.button("Delete").clicked() {
                    self.delete_todo(device_id, todo.id);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_todo_text);
            if ui.button("Add todo").clicked() && !self.new_todo_text.is_empty() {
                self.add_todo(device_id);
            }
        });
    }

    fn add_alarm(&mut self, device_id: i64) {
        let Some(client) = self.client() else { return };
        let req = server::UpsertAlarmRequest {
            hour: self.new_alarm_hour,
            minute: self.new_alarm_minute,
            repeat: server::Repeat::Daily,
            enabled: true,
            label: self.new_alarm_label.clone(),
        };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.create_alarm(device_id, &req) {
                Ok(()) => ServerEvent::ActionDone("Alarm added".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn toggle_alarm(&mut self, device_id: i64, mut alarm: server::Alarm) {
        let Some(client) = self.client() else { return };
        alarm.enabled = !alarm.enabled;
        let req = server::UpsertAlarmRequest {
            hour: alarm.hour,
            minute: alarm.minute,
            repeat: alarm.repeat,
            enabled: alarm.enabled,
            label: alarm.label,
        };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.update_alarm(device_id, alarm.id, &req) {
                Ok(()) => ServerEvent::ActionDone("Alarm updated".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn delete_alarm(&mut self, device_id: i64, alarm_id: u8) {
        let Some(client) = self.client() else { return };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.delete_alarm(device_id, alarm_id) {
                Ok(()) => ServerEvent::ActionDone("Alarm deleted".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn add_todo(&mut self, device_id: i64) {
        let Some(client) = self.client() else { return };
        let req = server::UpsertTodoRequest {
            text: self.new_todo_text.clone(),
            done: false,
        };
        self.new_todo_text.clear();
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.create_todo(device_id, &req) {
                Ok(()) => ServerEvent::ActionDone("Todo added".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn toggle_todo(&mut self, device_id: i64, mut todo: server::Todo) {
        let Some(client) = self.client() else { return };
        todo.done = !todo.done;
        let req = server::UpsertTodoRequest {
            text: todo.text,
            done: todo.done,
        };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.update_todo(device_id, todo.id, &req) {
                Ok(()) => ServerEvent::ActionDone("Todo updated".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }

    fn delete_todo(&mut self, device_id: i64, todo_id: u8) {
        let Some(client) = self.client() else { return };
        let tx = self.server_event_tx.clone();
        thread::spawn(move || {
            let event = match client.delete_todo(device_id, todo_id) {
                Ok(()) => ServerEvent::ActionDone("Todo deleted".to_string()),
                Err(err) => ServerEvent::Error(err.to_string()),
            };
            let _ = tx.send(event);
        });
    }
}

enum DeviceEvent {
    Usb(crate::transport::usb::UsbEvent),
    Ble(crate::transport::ble::BleEvent),
}
