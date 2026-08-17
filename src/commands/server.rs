//! Tauri commands for talking to `inkpaper-server`'s admin API. These
//! all run on a worker thread (`spawn_blocking`) since `reqwest` here
//! is in blocking mode (matches the existing `ServerClient`).
//!
//! The wire types come from `crate::server`. Error mapping goes
//! through `crate::error::from_reqwest` so 401/403 becomes
//! `SERVER_UNAUTHORIZED` rather than a generic unreachable.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::desktop::SharedState;
use crate::error::{from_reqwest, AppError};
use crate::server::{
    Alarm, Device, Repeat, ServerClient, Todo, UpsertAlarmRequest, UpsertTodoRequest,
};

fn client(base_url: String, token: String) -> Result<ServerClient, AppError> {
    let url = crate::commands::logs::normalise_server_url(&base_url)
        .ok_or_else(|| AppError::invalid_input("Server URL", "must not be empty"))?;
    Ok(ServerClient::new(url, token))
}

// ---------- Devices ----------

#[tauri::command]
pub async fn list_devices(
    base_url: String,
    token: String,
    state: State<'_, SharedState>,
) -> Result<Vec<Device>, AppError> {
    state
        .logs
        .info("server", format!("→ GET {base_url}/api/devices"));
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Device>, AppError> {
        let c = client(base_url, token)?;
        c.list_devices().map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("list_devices task: {e}")))?
}

#[tauri::command]
pub async fn register_device(
    base_url: String,
    token: String,
    name: String,
    state: State<'_, SharedState>,
) -> Result<Device, AppError> {
    state
        .logs
        .info("server", format!("→ POST {base_url}/api/devices name={name}"));
    tauri::async_runtime::spawn_blocking(move || -> Result<Device, AppError> {
        let c = client(base_url, token)?;
        c.register_device(&name).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("register_device task: {e}")))?
}

#[tauri::command]
pub async fn delete_device(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    state
        .logs
        .info("server", format!("→ DELETE {base_url}/api/devices/{device_id}"));
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        c.delete_device(device_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("delete_device task: {e}")))?
}

// ---------- Alarms ----------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInput {
    pub hour: u8,
    pub minute: u8,
    pub label: String,
    pub repeat: Repeat,
    pub enabled: bool,
}

#[tauri::command]
pub async fn list_alarms(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<Vec<Alarm>, AppError> {
    state.logs.info(
        "server",
        format!("→ GET {base_url}/api/devices/{device_id}/alarms"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Alarm>, AppError> {
        let c = client(base_url, token)?;
        c.list_alarms(device_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("list_alarms task: {e}")))?
}

#[tauri::command]
pub async fn create_alarm(
    base_url: String,
    token: String,
    device_id: i64,
    input: AlarmInput,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    validate_alarm(&input)?;
    state.logs.info(
        "server",
        format!(
            "→ POST {base_url}/api/devices/{device_id}/alarms {}:{:02}",
            input.hour, input.minute
        ),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        let req = UpsertAlarmRequest {
            hour: input.hour,
            minute: input.minute,
            label: input.label,
            repeat: input.repeat,
            enabled: input.enabled,
        };
        c.create_alarm(device_id, &req).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("create_alarm task: {e}")))?
}

#[tauri::command]
pub async fn update_alarm(
    base_url: String,
    token: String,
    device_id: i64,
    alarm_id: u8,
    input: AlarmInput,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    validate_alarm(&input)?;
    state.logs.info(
        "server",
        format!(
            "→ PUT {base_url}/api/devices/{device_id}/alarms/{alarm_id} {}:{:02}",
            input.hour, input.minute
        ),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        let req = UpsertAlarmRequest {
            hour: input.hour,
            minute: input.minute,
            label: input.label,
            repeat: input.repeat,
            enabled: input.enabled,
        };
        c.update_alarm(device_id, alarm_id, &req)
            .map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("update_alarm task: {e}")))?
}

#[tauri::command]
pub async fn delete_alarm(
    base_url: String,
    token: String,
    device_id: i64,
    alarm_id: u8,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    state.logs.info(
        "server",
        format!("→ DELETE {base_url}/api/devices/{device_id}/alarms/{alarm_id}"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        c.delete_alarm(device_id, alarm_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("delete_alarm task: {e}")))?
}

#[tauri::command]
pub async fn clear_alarms(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    state.logs.warn(
        "server",
        format!("→ DELETE {base_url}/api/devices/{device_id}/alarms (clear)"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        c.clear_alarms(device_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("clear_alarms task: {e}")))?
}

// ---------- Todos ----------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoInput {
    pub text: String,
    pub done: bool,
}

#[tauri::command]
pub async fn list_todos(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<Vec<Todo>, AppError> {
    state.logs.info(
        "server",
        format!("→ GET {base_url}/api/devices/{device_id}/todos"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Todo>, AppError> {
        let c = client(base_url, token)?;
        c.list_todos(device_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("list_todos task: {e}")))?
}

#[tauri::command]
pub async fn create_todo(
    base_url: String,
    token: String,
    device_id: i64,
    input: TodoInput,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    validate_todo(&input)?;
    state.logs.info(
        "server",
        format!(
            "→ POST {base_url}/api/devices/{device_id}/todos len={}",
            input.text.chars().count()
        ),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        let req = UpsertTodoRequest {
            text: input.text,
            done: input.done,
        };
        c.create_todo(device_id, &req).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("create_todo task: {e}")))?
}

#[tauri::command]
pub async fn update_todo(
    base_url: String,
    token: String,
    device_id: i64,
    todo_id: u8,
    input: TodoInput,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    validate_todo(&input)?;
    state.logs.info(
        "server",
        format!(
            "→ PUT {base_url}/api/devices/{device_id}/todos/{todo_id} done={}",
            input.done
        ),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        let req = UpsertTodoRequest {
            text: input.text,
            done: input.done,
        };
        c.update_todo(device_id, todo_id, &req).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("update_todo task: {e}")))?
}

#[tauri::command]
pub async fn delete_todo(
    base_url: String,
    token: String,
    device_id: i64,
    todo_id: u8,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    state.logs.info(
        "server",
        format!("→ DELETE {base_url}/api/devices/{device_id}/todos/{todo_id}"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        c.delete_todo(device_id, todo_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("delete_todo task: {e}")))?
}

#[tauri::command]
pub async fn clear_todos(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<(), AppError> {
    state.logs.warn(
        "server",
        format!("→ DELETE {base_url}/api/devices/{device_id}/todos (clear)"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let c = client(base_url, token)?;
        c.clear_todos(device_id).map_err(map_err)
    })
    .await
    .map_err(|e| AppError::internal(format!("clear_todos task: {e}")))?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSnapshot {
    pub alarms: Vec<Alarm>,
    pub todos: Vec<Todo>,
}

#[tauri::command]
pub async fn list_content(
    base_url: String,
    token: String,
    device_id: i64,
    state: State<'_, SharedState>,
) -> Result<ContentSnapshot, AppError> {
    state.logs.info(
        "server",
        format!("→ GET {base_url}/api/devices/{device_id}/(alarms+todos)"),
    );
    tauri::async_runtime::spawn_blocking(move || -> Result<ContentSnapshot, AppError> {
        let c = client(base_url, token)?;
        let alarms = c.list_alarms(device_id).map_err(map_err)?;
        let todos = c.list_todos(device_id).map_err(map_err)?;
        Ok(ContentSnapshot { alarms, todos })
    })
    .await
    .map_err(|e| AppError::internal(format!("list_content task: {e}")))?
}

// ---------- helpers ----------

fn map_err(e: anyhow::Error) -> AppError {
    match e.downcast::<reqwest::Error>() {
        Ok(req_err) => from_reqwest(req_err),
        Err(other) => AppError::server_unreachable(format!("{other:#}")),
    }
}

fn validate_alarm(input: &AlarmInput) -> Result<(), AppError> {
    if input.hour > 23 {
        return Err(AppError::invalid_input("hour", "must be 0..=23"));
    }
    if input.minute > 59 {
        return Err(AppError::invalid_input("minute", "must be 0..=59"));
    }
    if input.label.chars().count() > 32 {
        return Err(AppError::invalid_input("label", "longer than 32 chars"));
    }
    if let Repeat::Once { year, month, day } = input.repeat {
        if !(1900..=2200).contains(&year) {
            return Err(AppError::invalid_input("year", "out of range"));
        }
        if !(1..=12).contains(&month) {
            return Err(AppError::invalid_input("month", "must be 1..=12"));
        }
        if !(1..=31).contains(&day) {
            return Err(AppError::invalid_input("day", "must be 1..=31"));
        }
    }
    Ok(())
}

fn validate_todo(input: &TodoInput) -> Result<(), AppError> {
    let trimmed = input.text.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("text", "must not be empty"));
    }
    if input.text.chars().count() > 200 {
        return Err(AppError::invalid_input("text", "longer than 200 chars"));
    }
    Ok(())
}
