// TypeScript mirrors of the wire types defined in Rust. Field names use
// camelCase because that's what serde produces via `#[serde(rename_all
// = "camelCase")]` on every relevant struct/enum in src/.

/** Weekdays are 0=Sunday..6=Saturday; month days are 1..=31. */
export type Repeat =
  | "Daily"
  | { Weekly: { days: number[] } }
  | { Monthly: { days: number[] } }
  | { Once: { year: number; month: number; day: number } };

export interface Alarm {
  id: number;
  hour: number;
  minute: number;
  repeat: Repeat;
  enabled: boolean;
  label: string;
}

export type Importance = "low" | "medium" | "high";

export interface TodoDue {
  year: number;
  month: number;
  day: number;
}

export interface Todo {
  id: number;
  text: string;
  done: boolean;
  importance: Importance;
  dueDate: TodoDue | null;
  repeat: Repeat | null;
}

export interface Device {
  /** UUID string, opaque - not displayed in the UI. */
  id: string;
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

/** A nearby 2.4 GHz access point returned by the PC-side Wi-Fi scan. */
export interface WifiNetwork {
  ssid: string;
  channel: number;
  /** Signal strength as a 0-100 percentage, when the platform reports it. */
  signal?: number;
  security?: string;
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
  channels: Channel[];
  inbox: InboxItem[];
}

export interface Channel {
  id: string;
  deviceId: string;
  kind: string;
  name: string;
  enabled: boolean;
  tokenPrefix: string;
  lastSyncAt: number | null;
  lastSyncError: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface ChannelCreated {
  channel: Channel;
  token?: string | null;
  deliveryUrl?: string | null;
}

export type InboxKind = "alert" | "event" | "info";
export type InboxPriority = "normal" | "high";

export interface InboxItem {
  id: number;
  kind: InboxKind;
  priority: InboxPriority;
  title: string;
  body: string;
  when: number | null;
  read: boolean;
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
  importance: Importance;
  dueDate: TodoDue | null;
  repeat: Repeat | null;
}

export interface AppError {
  code: string;
  message: string;
  detail?: string;
}
