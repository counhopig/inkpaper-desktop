<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Frame from "../components/Frame.vue";
import StatusMark from "../components/StatusMark.vue";
import Button from "../components/Button.vue";
import Notice from "../components/Notice.vue";
import EmptyState from "../components/EmptyState.vue";
import { useDeviceStore } from "../stores/device";
import { useServerStore } from "../stores/server";
import { useLogsStore } from "../stores/logs";
import { formatTimeShort, formatUtcOffset } from "../lib/format";
import { getDeviceStatus, syncNow as syncNowCmd } from "../lib/commands";

const emit = defineEmits<{ (e: "navigate", page: "overview" | "device" | "content" | "logs"): void }>();

const device = useDeviceStore();
const server = useServerStore();
const logs = useLogsStore();

const refreshing = ref(false);
const refreshingStatus = ref(false);

async function refresh() {
  refreshing.value = true;
  try {
    await device.refreshPorts();
    if (device.isConnected) {
      refreshingStatus.value = true;
      const r = await device.run("status", getDeviceStatus);
      if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
      refreshingStatus.value = false;
    }
    await server.refreshDevices();
  } finally {
    refreshing.value = false;
  }
}

async function syncNow() {
  await device.run("sync", syncNowCmd);
}

const status = computed(() => {
  if (!device.connection.connected) return "idle" as const;
  if (device.ops["sync-status"]?.state === "error") return "warn" as const;
  return "ok" as const;
});

const lastReplyDisplay = computed(() => {
  if (!device.lastReplyAt) return "—";
  return formatTimeShort(device.lastReplyAt);
});

const setupSteps = computed(() => {
  const ds = device.deviceStatus;
  const steps = [
    {
      label: "Device connection",
      done: device.isConnected,
      detail: device.connection.kind + " · " + device.connection.port,
    },
    {
      label: "Wi-Fi credentials",
      done: ds?.wifiConfigured ?? false,
      detail: ds?.wifiConnected ? "connected" : ds?.wifiConfigured ? "stored, not connected" : "not set",
    },
    {
      label: "Public sync endpoint",
      done: ds?.serverConfigured ?? false,
      detail: ds?.serverConfigured ? "device has a sync URL" : "not set",
    },
    {
      label: "Timezone",
      // The device always reports a timezone offset (defaults to 0 = UTC),
      // so this step completes once a status reply has been received - i.e.
      // the device is reachable and its clock offset is known. There is no
      // "has the user explicitly set it" flag in the protocol.
      done: ds?.timezoneOffsetMinutes != null,
      detail:
        ds?.timezoneOffsetMinutes != null
          ? formatUtcOffset(ds.timezoneOffsetMinutes)
          : "set on the Device page",
    },
    {
      label: "Desktop server access",
      done: server.connected,
      detail: server.connected ? server.baseUrl || "linked" : "open Content page to configure",
    },
  ];
  const firstPending = steps.findIndex((s) => !s.done);
  return steps.map((s, i) => ({ ...s, current: i === firstPending }));
});

const recent = computed(() => logs.entries.slice(-12).reverse());

// When a device connects (or is already connected when Overview opens),
// pull its status once so the mini-screen shows live values instead of a
// blank "offline". device.bootstrap() resolves after this component mounts,
// so react to the connection flag rather than probing on mount.
watch(
  () => device.isConnected,
  async (connected) => {
    if (connected && !device.deviceStatus) {
      const r = await device.run("status", getDeviceStatus);
      if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
    }
  },
  { immediate: true },
);

const screenTime = computed(() =>
  new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
);

/** The next enabled alarm today (or the earliest one if all have passed). */
const nextAlarmText = computed(() => {
  const enabled = server.alarms.filter((a) => a.enabled);
  if (enabled.length === 0) return "—";
  const now = new Date();
  const cur = now.getHours() * 60 + now.getMinutes();
  const sorted = [...enabled].sort((a, b) => a.hour * 60 + a.minute - (b.hour * 60 + b.minute));
  const next = sorted.find((a) => a.hour * 60 + a.minute > cur) ?? sorted[0];
  return `${String(next.hour).padStart(2, "0")}:${String(next.minute).padStart(2, "0")}`;
});
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div>
        <h1 class="page-title">Overview</h1>
        <p class="page-subtitle">Where things stand right now</p>
      </div>
      <div class="row end">
        <Button variant="ghost" :loading="refreshing" @click="refresh">Refresh</Button>
        <Button
          variant="primary"
          :loading="!!device.ops.sync?.state && device.ops.sync.state === 'running'"
          :disabled="!device.isConnected"
          @click="syncNow"
        >
          Sync device now
        </Button>
      </div>
    </header>

    <div class="page-grid">
      <div class="col-6">
        <Frame title="Device status" style="--i: 1">
          <div class="ink-screen" aria-hidden="true">
            <div class="ink-screen-top">
              <span class="ink-screen-time">{{ screenTime }}</span>
              <span class="ink-screen-batt">◼◼◼◼</span>
            </div>
            <div class="ink-screen-row">
              <span class="lbl">Wi-Fi</span>
              <span class="val">{{ device.deviceStatus?.wifiConnected ? "◉ " + (device.deviceStatus.wifiSsid ?? "connected") : "○ offline" }}</span>
            </div>
            <div class="ink-screen-row">
              <span class="lbl">Sync</span>
              <span class="val">{{ device.deviceStatus?.serverConfigured ? "● configured" : "○ not set" }}</span>
            </div>
            <div class="ink-screen-row">
              <span class="lbl">Next</span>
              <span class="val">▸ {{ nextAlarmText }}</span>
            </div>
            <div class="ink-screen-row">
              <span class="lbl">Todo</span>
              <span class="val">▸ {{ server.todoDoneCount }} / {{ server.todoCount }}</span>
            </div>
          </div>
          <div class="kv">
            <div class="k">Connection</div>
            <div class="v"><StatusMark :status="status" :label="device.connection.kind" /></div>
            <div class="k">Port</div>
            <div class="v">{{ device.connection.port }}</div>
            <div class="k">Wi-Fi</div>
            <div class="v">
              <span v-if="device.deviceStatus?.wifiConfigured">
                configured<template v-if="device.deviceStatus.wifiConnected"> · connected</template>
              </span>
              <span v-else>—</span>
            </div>
            <div class="k">Sync server</div>
            <div class="v">{{ device.deviceStatus?.serverConfigured ? "configured" : "—" }}</div>
            <div class="k">Last reply</div>
            <div class="v">{{ lastReplyDisplay }}</div>
          </div>
          <template v-if="device.ops.sync?.state === 'error'">
            <Notice variant="error" :title="device.ops.sync.errorCode ?? 'Sync failed'">
              {{ device.ops.sync.errorMessage }}
            </Notice>
          </template>
          <div class="row between" style="margin-top: var(--s-2);">
            <Button variant="ghost" size="small" @click="emit('navigate', 'device')">Configure device</Button>
            <Button
              variant="ghost"
              size="small"
              :loading="refreshingStatus"
              :disabled="!device.isConnected"
              @click="async () => {
                const r = await device.run('status', getDeviceStatus);
                if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
              }"
            >
              Refresh status
            </Button>
          </div>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Server status" style="--i: 2">
          <div class="stat-blocks">
            <div class="stat-block">
              <div class="num">{{ server.alarmCount }}</div>
              <div class="cap">Alarms</div>
            </div>
            <div class="stat-block">
              <div class="num">{{ server.todoDoneCount }}</div>
              <div class="cap">Done</div>
            </div>
            <div class="stat-block">
              <div class="num">{{ server.todoCount }}</div>
              <div class="cap">Todos</div>
            </div>
          </div>
          <div class="kv">
            <div class="k">URL</div>
            <div class="v" style="font-family: var(--font-mono);">{{ server.baseUrl || "—" }}</div>
            <div class="k">Linked</div>
            <div class="v">
              <StatusMark :status="server.connected ? 'ok' : 'idle'" :label="server.connected ? 'linked' : 'not linked'" />
            </div>
            <div class="k">Selected device</div>
            <div class="v">{{ server.selectedDevice ? `${server.selectedDevice.name}` : "—" }}</div>
          </div>
          <template v-if="server.lastError">
            <Notice variant="error" :title="server.lastError.code">
              {{ server.lastError.message }}
            </Notice>
          </template>
          <div class="row end" style="margin-top: var(--s-2);">
            <Button variant="ghost" size="small" @click="emit('navigate', 'content')">Manage content</Button>
          </div>
        </Frame>
      </div>

      <div class="col-8">
        <Frame title="Setup progress" style="--i: 3">
          <ol class="step-list">
            <li
              v-for="(s, idx) in setupSteps"
              :key="idx"
              :class="['step', { done: s.done, current: s.current }]"
            >
              <span class="num">{{ s.done ? "✓" : idx + 1 }}</span>
              <span class="label">{{ s.label }}</span>
              <span class="meta">
                <span v-if="s.done" class="mark ok">Done</span>
                <span v-else-if="s.current" class="mark pending">{{ s.detail }}</span>
                <span v-else class="mark idle">{{ s.detail }}</span>
              </span>
            </li>
          </ol>
        </Frame>
      </div>

      <div class="col-4">
        <Frame title="Recent activity" style="--i: 4">
          <EmptyState v-if="recent.length === 0" glyph="·" title="No activity yet">
            Connect a device or run a sync to populate this list.
          </EmptyState>
          <div v-else class="row-list">
            <div v-for="e in recent" :key="e.timestampMs + e.message" class="row-item compact">
              <div class="body">
                <div class="title">{{ e.source }}</div>
                <div class="meta">{{ formatTimeShort(e.timestampMs) }} · {{ e.message }}</div>
              </div>
              <span :class="['mark', e.level === 'error' ? 'fail' : e.level === 'warn' ? 'warn' : 'idle']">{{ e.level }}</span>
            </div>
          </div>
        </Frame>
      </div>
    </div>
  </div>
</template>
