// Front-end validators that mirror the Rust-side checks in
// `commands::server::validate_alarm` / `validate_todo` and
// `commands::device::set_*`. Errors returned here are surfaced
// verbatim in the UI; the Rust side will reject anything we miss, so
// these exist for fast feedback, not for security.

import type { AlarmInput, Repeat, TodoInput } from "./types";

export type FieldErrors<T> = Partial<Record<keyof T | "_", string>>;

const SSID_MAX = 32;
const LABEL_MAX = 32;
const TODO_TEXT_MAX = 200;

export function validateSsid(ssid: string): string | null {
  if (!ssid.trim()) return "SSID must not be empty";
  if (ssid.length > SSID_MAX) return `SSID longer than ${SSID_MAX} chars`;
  return null;
}

export function validatePassword(pwd: string): string | null {
  // Open networks are allowed: empty password is valid.
  if (pwd.length > 63) return "Wi-Fi passwords are at most 63 chars";
  return null;
}

export function validateUrl(url: string): string | null {
  if (!url.trim()) return "URL must not be empty";
  return null;
}

export function validateToken(token: string, opts: { allowEmpty?: boolean } = {}): string | null {
  const trimmed = token.trim();
  if (!trimmed && !opts.allowEmpty) return "Token must not be empty";
  return null;
}

export function validateAlarm(input: AlarmInput): FieldErrors<AlarmInput> {
  const errs: FieldErrors<AlarmInput> = {};
  if (input.hour < 0 || input.hour > 23) errs.hour = "0..23";
  if (input.minute < 0 || input.minute > 59) errs.minute = "0..59";
  if (input.label.length > LABEL_MAX) errs.label = `longer than ${LABEL_MAX} chars`;
  if (typeof input.repeat === "object" && "Once" in input.repeat) {
    const { year, month, day } = input.repeat.Once;
    if (year < 1900 || year > 2200) errs.repeat = "year out of range";
    else if (month < 1 || month > 12) errs.repeat = "month must be 1..12";
    else if (day < 1 || day > 31) errs.repeat = "day must be 1..31";
  }
  return errs;
}

export function validateTodo(input: TodoInput): FieldErrors<TodoInput> {
  const errs: FieldErrors<TodoInput> = {};
  if (!input.text.trim()) errs.text = "must not be empty";
  if (input.text.length > TODO_TEXT_MAX) errs.text = `longer than ${TODO_TEXT_MAX} chars`;
  return errs;
}

export function validateTimezone(offset: number): string | null {
  if (!Number.isFinite(offset)) return "not a number";
  if (offset < -12 * 60 || offset > 14 * 60) return "out of range";
  if (offset % 15 !== 0) return "must be a multiple of 15 minutes";
  return null;
}

export function isAlarmFormValid(input: AlarmInput): boolean {
  return Object.keys(validateAlarm(input)).length === 0;
}

export function isTodoFormValid(input: TodoInput): boolean {
  return Object.keys(validateTodo(input)).length === 0;
}

export function repeatLabel(repeat: Repeat): string {
  if (repeat === "Daily") return "Daily";
  const { year, month, day } = repeat.Once;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `Once · ${year}-${pad(month)}-${pad(day)}`;
}
