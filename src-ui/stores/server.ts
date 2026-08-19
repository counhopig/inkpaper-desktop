// Server-side admin API state: URL + admin token, registered devices,
// currently selected device, and the alarms/todos for that device.
// Tokens are kept in localStorage for now (see `lib/storage.ts`).

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import * as C from "../lib/commands";
import {
  loadAdminToken,
  loadSelectedDeviceId,
  loadServerBaseUrl,
  saveAdminToken,
  saveSelectedDeviceId,
  saveServerBaseUrl,
} from "../lib/storage";
import type { Alarm, AlarmInput, Device, Todo, TodoInput } from "../lib/types";

export const useServerStore = defineStore("server", () => {
  const baseUrl = ref(loadServerBaseUrl());
  const adminToken = ref(loadAdminToken());
  const connected = ref(false);
  const lastError = ref<{ code: string; message: string } | null>(null);

  const devices = ref<Device[]>([]);
  const selectedDeviceId = ref<string | null>(loadSelectedDeviceId());
  const alarms = ref<Alarm[]>([]);
  const todos = ref<Todo[]>([]);

  function setBaseUrl(value: string) {
    baseUrl.value = value;
    saveServerBaseUrl(value);
    connected.value = false;
  }

  function setAdminToken(value: string) {
    adminToken.value = value;
    saveAdminToken(value);
    connected.value = false;
  }

  function selectDevice(id: string | null) {
    selectedDeviceId.value = id;
    saveSelectedDeviceId(id);
    alarms.value = [];
    todos.value = [];
  }

  async function refreshDevices() {
    if (!baseUrl.value.trim()) {
      connected.value = false;
      return { ok: false as const, error: { code: "INVALID_INPUT", message: "No server URL configured" } };
    }
    const r = await C.listDevices(baseUrl.value, adminToken.value);
    if (r.ok) {
      devices.value = r.value;
      connected.value = true;
      lastError.value = null;
      // A device selected in an earlier session (persisted to localStorage)
      // may no longer exist on this server - e.g. its DB was reset, or the
      // base URL now points at a different server. Drop the stale
      // selection instead of leaving it referencing a nonexistent device,
      // which would otherwise let alarm/todo writes hit a foreign key
      // that no longer resolves.
      if (
        selectedDeviceId.value != null &&
        !r.value.some((d) => d.id === selectedDeviceId.value)
      ) {
        selectDevice(null);
      }
    } else {
      lastError.value = { code: r.error.code, message: r.error.message };
      connected.value = false;
    }
    return r;
  }

  async function registerDevice(name: string) {
    const r = await C.registerDevice(baseUrl.value, adminToken.value, name);
    if (r.ok) {
      devices.value = [...devices.value, r.value];
    } else {
      lastError.value = { code: r.error.code, message: r.error.message };
    }
    return r;
  }

  async function deleteDevice(id: string) {
    const r = await C.deleteDevice(baseUrl.value, adminToken.value, id);
    if (r.ok) {
      devices.value = devices.value.filter((d) => d.id !== id);
      if (selectedDeviceId.value === id) selectDevice(null);
    } else {
      lastError.value = { code: r.error.code, message: r.error.message };
    }
    return r;
  }

  async function refreshContent() {
    if (selectedDeviceId.value == null) return;
    const r = await C.listContent(baseUrl.value, adminToken.value, selectedDeviceId.value);
    if (r.ok) {
      alarms.value = r.value.alarms;
      todos.value = r.value.todos;
      connected.value = true;
      lastError.value = null;
    } else {
      lastError.value = { code: r.error.code, message: r.error.message };
      connected.value = false;
    }
    return r;
  }

  async function createAlarm(input: AlarmInput) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.createAlarm(baseUrl.value, adminToken.value, selectedDeviceId.value, input);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function updateAlarm(id: number, input: AlarmInput) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.updateAlarm(baseUrl.value, adminToken.value, selectedDeviceId.value, id, input);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function deleteAlarm(id: number) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.deleteAlarm(baseUrl.value, adminToken.value, selectedDeviceId.value, id);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function clearAlarms() {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.clearAlarms(baseUrl.value, adminToken.value, selectedDeviceId.value);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function createTodo(input: TodoInput) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.createTodo(baseUrl.value, adminToken.value, selectedDeviceId.value, input);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function updateTodo(id: number, input: TodoInput) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.updateTodo(baseUrl.value, adminToken.value, selectedDeviceId.value, id, input);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function deleteTodo(id: number) {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.deleteTodo(baseUrl.value, adminToken.value, selectedDeviceId.value, id);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  async function clearTodos() {
    if (selectedDeviceId.value == null) return { ok: false as const, error: { code: "INVALID_INPUT", message: "No device selected" } };
    const r = await C.clearTodos(baseUrl.value, adminToken.value, selectedDeviceId.value);
    if (r.ok) await refreshContent();
    else lastError.value = { code: r.error.code, message: r.error.message };
    return r;
  }

  const selectedDevice = computed(() =>
    devices.value.find((d) => d.id === selectedDeviceId.value) ?? null,
  );

  const alarmCount = computed(() => alarms.value.length);
  const todoCount = computed(() => todos.value.length);
  const todoDoneCount = computed(() => todos.value.filter((t) => t.done).length);

  return {
    baseUrl,
    adminToken,
    connected,
    lastError,
    devices,
    selectedDeviceId,
    selectedDevice,
    alarms,
    todos,
    alarmCount,
    todoCount,
    todoDoneCount,
    setBaseUrl,
    setAdminToken,
    selectDevice,
    refreshDevices,
    registerDevice,
    deleteDevice,
    refreshContent,
    createAlarm,
    updateAlarm,
    deleteAlarm,
    clearAlarms,
    createTodo,
    updateTodo,
    deleteTodo,
    clearTodos,
  };
});
