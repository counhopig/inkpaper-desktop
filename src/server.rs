//! HTTP client for `inkpaper-server`'s admin API (device registration,
//! alarm/todo management). Mirrors the wire types in
//! `inkpaper-server/src/models.rs`, which in turn mirror the firmware's
//! `alarms::StoredAlarm`/`todos::Todo` - three independent copies of the
//! same shape across three repos/languages-in-spirit, kept honest by the
//! shared JSON contract rather than shared code.
//!
//! Every call here is a blocking `reqwest` request; callers (the egui app)
//! must run these on a background thread, not directly in `update()`, or
//! the UI will freeze for the duration of the request.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Repeat {
    Daily,
    Once { year: u16, month: u8, day: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    pub id: u8,
    pub hour: u8,
    pub minute: u8,
    pub repeat: Repeat,
    pub enabled: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: u8,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpsertAlarmRequest {
    pub hour: u8,
    pub minute: u8,
    pub repeat: Repeat,
    pub enabled: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpsertTodoRequest {
    pub text: String,
    pub done: bool,
}

#[derive(Clone)]
pub struct ServerClient {
    base_url: String,
    admin_token: String,
    client: reqwest::blocking::Client,
}

impl ServerClient {
    pub fn new(base_url: String, admin_token: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            admin_token,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth(&self, builder: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        builder.bearer_auth(&self.admin_token)
    }

    pub fn register_device(&self, name: &str) -> anyhow::Result<Device> {
        let resp = self
            .auth(self.client.post(self.url("/api/devices")))
            .json(&serde_json::json!({ "name": name }))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn list_devices(&self) -> anyhow::Result<Vec<Device>> {
        let resp = self
            .auth(self.client.get(self.url("/api/devices")))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn delete_device(&self, id: i64) -> anyhow::Result<()> {
        self.auth(self.client.delete(self.url(&format!("/api/devices/{id}"))))
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn list_alarms(&self, device_id: i64) -> anyhow::Result<Vec<Alarm>> {
        let resp = self
            .auth(self.client.get(self.url(&format!("/api/devices/{device_id}/alarms"))))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn create_alarm(&self, device_id: i64, req: &UpsertAlarmRequest) -> anyhow::Result<()> {
        self.auth(self.client.post(self.url(&format!("/api/devices/{device_id}/alarms"))))
            .json(req)
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn update_alarm(&self, device_id: i64, alarm_id: u8, req: &UpsertAlarmRequest) -> anyhow::Result<()> {
        self.auth(
            self.client
                .put(self.url(&format!("/api/devices/{device_id}/alarms/{alarm_id}"))),
        )
        .json(req)
        .send()?
        .error_for_status()?;
        Ok(())
    }

    pub fn delete_alarm(&self, device_id: i64, alarm_id: u8) -> anyhow::Result<()> {
        self.auth(
            self.client
                .delete(self.url(&format!("/api/devices/{device_id}/alarms/{alarm_id}"))),
        )
        .send()?
        .error_for_status()?;
        Ok(())
    }

    pub fn list_todos(&self, device_id: i64) -> anyhow::Result<Vec<Todo>> {
        let resp = self
            .auth(self.client.get(self.url(&format!("/api/devices/{device_id}/todos"))))
            .send()?
            .error_for_status()?;
        Ok(resp.json()?)
    }

    pub fn create_todo(&self, device_id: i64, req: &UpsertTodoRequest) -> anyhow::Result<()> {
        self.auth(self.client.post(self.url(&format!("/api/devices/{device_id}/todos"))))
            .json(req)
            .send()?
            .error_for_status()?;
        Ok(())
    }

    pub fn update_todo(&self, device_id: i64, todo_id: u8, req: &UpsertTodoRequest) -> anyhow::Result<()> {
        self.auth(
            self.client
                .put(self.url(&format!("/api/devices/{device_id}/todos/{todo_id}"))),
        )
        .json(req)
        .send()?
        .error_for_status()?;
        Ok(())
    }

    pub fn delete_todo(&self, device_id: i64, todo_id: u8) -> anyhow::Result<()> {
        self.auth(
            self.client
                .delete(self.url(&format!("/api/devices/{device_id}/todos/{todo_id}"))),
        )
        .send()?
        .error_for_status()?;
        Ok(())
    }
}
