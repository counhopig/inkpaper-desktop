<script setup lang="ts">
import { computed, ref } from "vue";
import Frame from "../components/Frame.vue";
import StatusMark from "../components/StatusMark.vue";
import Button from "../components/Button.vue";
import Notice from "../components/Notice.vue";
import EmptyState from "../components/EmptyState.vue";
import { useDeviceStore } from "../stores/device";
import { useServerStore } from "../stores/server";
import { useLogsStore } from "../stores/logs";
import { formatTimeShort } from "../lib/format";
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
      done: false,
      detail: "set on the Device page",
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
        <Frame title="Device status">
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
        <Frame title="Server status">
          <div class="kv">
            <div class="k">URL</div>
            <div class="v" style="font-family: var(--font-mono);">{{ server.baseUrl || "—" }}</div>
            <div class="k">Linked</div>
            <div class="v">
              <StatusMark :status="server.connected ? 'ok' : 'idle'" :label="server.connected ? 'linked' : 'not linked'" />
            </div>
            <div class="k">Selected device</div>
            <div class="v">{{ server.selectedDevice ? `${server.selectedDevice.name}` : "—" }}</div>
            <div class="k">Alarms</div>
            <div class="v">{{ server.alarmCount }}</div>
            <div class="k">Todos</div>
            <div class="v">{{ server.todoDoneCount }} / {{ server.todoCount }} done</div>
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
        <Frame title="Setup progress">
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
        <Frame title="Recent activity">
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
