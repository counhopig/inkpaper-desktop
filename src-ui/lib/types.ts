// TypeScript mirrors of the wire types defined in Rust. Field names use
// camelCase because that's what serde produces via `#[serde(rename_all
// = "camelCase")]` on every relevant struct/enum in src/.

export type Repeat =
  | "Daily"
  | { Once: { year: number; month: number; day: number } };

export interface Alarm {
  id: number;
  hour: number;
  minute: number;
  repeat: Repeat;
  enabled: boolean;
  label: string;
}

export interface Todo {
  id: number;
  text: string;
  done: boolean;
}

export interface Device {
  id: number;
  name: string;
  /** Only present on the response of `register_device`. */
  token?: string | null;
}

export type ConnectionKind = "Offline" | "USB" | "BLE";

export interface ConnectionStateInfo {
  connected: boolean;
  kind: ConnectionKind;
  port: string;
}

export interface DeviceStatus {
  wifiConfigured: boolean;
  serverConfigured: boolean;
  wifiConnected: boolean;
  wifiSsid?: string;
  wifiHasPassword?: boolean;
  serverUrl?: string;
  serverHasToken?: boolean;
  timezoneOffsetMinutes?: number;
}

export interface DeviceCommandResult {
  kind: "ok" | "status" | "error";
  message: string;
  status?: DeviceStatus;
}

export type LogLevel = "info" | "warn" | "error";

export interface LogEntry {
  timestampMs: number;
  level: LogLevel;
  source: string;
  message: string;
}

export interface ContentSnapshot {
  alarms: Alarm[];
  todos: Todo[];
}

export interface AlarmInput {
  hour: number;
  minute: number;
  label: string;
  repeat: Repeat;
  enabled: boolean;
}

export interface TodoInput {
  text: string;
  done: boolean;
}

export interface AppError {
  code: string;
  message: string;
  detail?: string;
}
