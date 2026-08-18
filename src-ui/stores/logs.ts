// Log buffer + filters for the Logs page. Subscribes to the
// `device-log` Tauri event for real-time appends so the UI never has
// to poll every 100ms.

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { listen } from "@tauri-apps/api/event";
import * as C from "../lib/commands";
import type { LogEntry, LogLevel } from "../lib/types";

export type LogFilter = "all" | "device" | "server" | "error";

export const useLogsStore = defineStore("logs", () => {
  const entries = ref<LogEntry[]>([]);
  const filter = ref<LogFilter>("all");
  const query = ref("");
  const paused = ref(false);
  const filePath = ref<string>("");
  const dirPath = ref<string>("");
  let pending: LogEntry[] = [];

  function flushPending() {
    if (pending.length === 0) return;
    entries.value = [...entries.value, ...pending];
    pending = [];
  }

  async function bootstrap() {
    const r = await C.readLogs();
    if (r.ok) entries.value = r.value;
    const fp = await C.logFilePath();
    if (fp.ok) filePath.value = fp.value;
    const dp = await C.logDir();
    if (dp.ok) dirPath.value = dp.value;
    try {
      await listen<LogEntry>("device-log", (e) => {
        pending.push(e.payload);
        if (!paused.value) {
          flushPending();
        }
      });
    } catch {
      // ignore in non-Tauri environments
    }
  }

  async function clear() {
    const r = await C.clearLogs();
    if (r.ok) entries.value = [];
  }

  function flush() {
    flushPending();
  }

  function visible(): LogEntry[] {
    let list: LogEntry[];
    if (filter.value === "all") {
      list = entries.value;
    } else if (filter.value === "error") {
      list = entries.value.filter((e) => e.level === "error");
    } else if (filter.value === "device") {
      list = entries.value.filter((e) => e.source.startsWith("device"));
    } else {
      list = entries.value.filter((e) => e.source.startsWith("server"));
    }
    const q = query.value.trim().toLowerCase();
    if (q) {
      list = list.filter(
        (e) =>
          e.message.toLowerCase().includes(q) ||
          e.source.toLowerCase().includes(q),
      );
    }
    return list;
  }

  const visibleEntries = computed(() => visible());

  const summary = computed(() => {
    const counts: Record<LogLevel, number> = { info: 0, warn: 0, error: 0 };
    for (const e of entries.value) counts[e.level]++;
    return counts;
  });

  return {
    entries,
    filter,
    query,
    paused,
    filePath,
    dirPath,
    visibleEntries,
    summary,
    bootstrap,
    clear,
    flush,
  };
});
