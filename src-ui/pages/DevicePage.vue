<script setup lang="ts">
import { computed, ref, onMounted, watch } from "vue";
import Frame from "../components/Frame.vue";
import Field from "../components/Field.vue";
import Button from "../components/Button.vue";
import Notice from "../components/Notice.vue";
import StatusMark from "../components/StatusMark.vue";
import EmptyState from "../components/EmptyState.vue";
import { useDeviceStore } from "../stores/device";
import {
  listCommonTimezones,
  systemTimezoneOffsetMinutes,
  formatUtcOffset,
  tzLabel,
} from "../lib/format";
import { validatePassword, validateSsid, validateUrl, validateToken, validateTimezone } from "../lib/validation";
import {
  getDeviceStatus,
  setWifi,
  setServer,
  setTimezone,
  syncNow,
  clearDeviceAlarms,
} from "../lib/commands";

const device = useDeviceStore();
const refreshing = ref(false);

const selectedPort = ref("");
const ssid = ref("");
const password = ref("");
const showPassword = ref(false);
const serverUrl = ref("");
const serverToken = ref("");
const tzOffset = ref<number>(systemTimezoneOffsetMinutes());

watch(
  () => device.usbPorts,
  (ports) => {
    if (!selectedPort.value && ports.length > 0) selectedPort.value = ports[0];
  },
  { immediate: true },
);

const ssidError = computed(() => (ssid.value ? validateSsid(ssid.value) : null));
const passwordError = computed(() => (password.value ? validatePassword(password.value) : null));
const urlError = computed(() => (serverUrl.value ? validateUrl(serverUrl.value) : null));
const tokenError = computed(() => validateToken(serverToken.value));
const tzError = computed(() => validateTimezone(tzOffset.value));

const connectingUsb = computed(() => device.ops.connect?.state === "running");
const bleScanning = computed(() => device.ops.bleScan?.state === "running");

onMounted(async () => {
  await device.refreshPorts();
});

async function connectUsb() {
  if (!selectedPort.value) return;
  await device.connectUsb(selectedPort.value);
  const r = await device.run("status", getDeviceStatus);
  if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
}

async function disconnect() {
  await device.disconnect();
}

async function scanBle() {
  const found = await device.discoverBle();
  if (found) {
    await device.connectBle();
    const r = await device.run("status", getDeviceStatus);
    if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
  }
}

async function applyWifi() {
  if (ssidError.value || passwordError.value) return;
  const r = await device.run("wifi", () => setWifi(ssid.value.trim(), password.value));
  if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
}

async function applyServer() {
  if (urlError.value || tokenError.value) return;
  const r = await device.run("server", () => setServer(serverUrl.value.trim(), serverToken.value.trim()));
  if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
}

async function applyTimezone() {
  if (tzError.value) return;
  const r = await device.run("timezone", () => setTimezone(tzOffset.value));
  if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
}

async function runSync() {
  await device.run("sync", syncNow);
}

async function refreshStatus() {
  refreshing.value = true;
  try {
    const r = await device.run("status", getDeviceStatus);
    if (r.ok && r.value) device.setDeviceStatusFromCommand(r.value);
  } finally {
    refreshing.value = false;
  }
}

async function clearAlarms() {
  await device.run("clear-alarms", clearDeviceAlarms);
  await refreshStatus();
}

const tzChoices = computed(() =>
  listCommonTimezones().map((t) => ({ ...t, label: tzLabel(t.name, t.offset) })),
);
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div>
        <h1 class="page-title">Device</h1>
        <p class="page-subtitle">Connect over USB or BLE and push Wi-Fi, server, timezone config.</p>
      </div>
      <div class="row end">
        <Button
          variant="ghost"
          :loading="refreshing"
          :disabled="!device.isConnected"
          @click="refreshStatus"
        >
          Refresh status
        </Button>
        <Button
          variant="primary"
          :loading="device.ops.sync?.state === 'running'"
          :disabled="!device.isConnected"
          @click="runSync"
        >
          Sync device now
        </Button>
      </div>
    </header>

    <div class="page-grid">
      <div class="col-6">
        <Frame title="Connection" :subtitle="device.connection.kind">
          <div class="stack">
            <div class="field-row">
              <div class="field" style="flex: 2;">
                <label>USB port</label>
                <select v-model="selectedPort" :disabled="connectingUsb || device.isConnected">
                  <option v-if="device.usbPorts.length === 0" value="">No USB serial devices detected</option>
                  <option v-for="p in device.usbPorts" :key="p" :value="p">{{ p }}</option>
                </select>
                <div class="hint">Espressif VID 0x303a ports are listed first.</div>
              </div>
              <div class="row end" style="flex: 1;">
                <Button v-if="!device.isConnected" variant="primary" :loading="connectingUsb" :disabled="!selectedPort" @click="connectUsb">Connect USB</Button>
                <Button v-else variant="danger" @click="disconnect">Disconnect</Button>
              </div>
            </div>

            <div class="frame-section">
              <div class="row between">
                <div>
                  <h3 style="margin:0; font-size: var(--t-14); font-weight: 600;">Bluetooth</h3>
                  <div class="hint">The Inkpaper only advertises while its BLE Pairing screen is open.</div>
                </div>
                <Button :loading="bleScanning" :disabled="device.isConnected" @click="scanBle">
                  {{ device.isConnected ? "Connected" : "Scan for Inkpaper" }}
                </Button>
              </div>
            </div>
          </div>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Device status">
          <div v-if="!device.isConnected">
            <EmptyState glyph="○" title="Not connected">
              Connect a device over USB or BLE to see live status.
            </EmptyState>
          </div>
          <div v-else class="kv">
            <div class="k">Transport</div>
            <div class="v">{{ device.connection.kind }} · {{ device.connection.port }}</div>
            <div class="k">Wi-Fi configured</div>
            <div class="v"><StatusMark :status="device.deviceStatus?.wifiConfigured ? 'ok' : 'idle'" :label="device.deviceStatus?.wifiConfigured ? 'yes' : 'no'" /></div>
            <div class="k">Wi-Fi connected</div>
            <div class="v"><StatusMark :status="device.deviceStatus?.wifiConnected ? 'ok' : 'idle'" :label="device.deviceStatus?.wifiConnected ? 'yes' : 'no'" /></div>
            <div class="k">Sync server</div>
            <div class="v"><StatusMark :status="device.deviceStatus?.serverConfigured ? 'ok' : 'idle'" :label="device.deviceStatus?.serverConfigured ? 'configured' : 'not set'" /></div>
          </div>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Wi-Fi" subtitle="Pushed to the device">
          <Field label="SSID" :error="ssidError ?? undefined">
            <input v-model="ssid" type="text" placeholder="Network name" maxlength="32" />
          </Field>
          <Field label="Password" :error="passwordError ?? undefined" :hint="!password ? 'Empty password is allowed for open networks.' : undefined">
            <div class="row" style="gap: 0;">
              <input
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                placeholder="(empty for open networks)"
                style="flex: 1;"
                maxlength="63"
              />
              <Button size="small" variant="ghost" @click="showPassword = !showPassword">
                {{ showPassword ? "Hide" : "Show" }}
              </Button>
            </div>
          </Field>
          <div class="row end">
            <Button
              variant="primary"
              :loading="device.ops.wifi?.state === 'running'"
              :disabled="!device.isConnected || !!ssidError || !!passwordError"
              @click="applyWifi"
            >
              Save Wi-Fi
            </Button>
          </div>
          <template v-if="device.ops.wifi?.state === 'error'">
            <Notice variant="error" :title="device.ops.wifi.errorCode ?? 'Failed'">
              {{ device.ops.wifi.errorMessage }}
            </Notice>
          </template>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Sync server" subtitle="Public URL + device token">
          <Field label="Server URL" :error="urlError ?? undefined">
            <input v-model="serverUrl" type="text" placeholder="http://192.168.1.10:8080" />
          </Field>
          <Field label="Device token" :error="tokenError ?? undefined" hint="Issued by the server when you register the device. Not the Admin Token.">
            <input v-model="serverToken" type="text" placeholder="paste device token here" />
          </Field>
          <div class="row end">
            <Button
              variant="primary"
              :loading="device.ops.server?.state === 'running'"
              :disabled="!device.isConnected || !!urlError || !!tokenError"
              @click="applyServer"
            >
              Save server config
            </Button>
          </div>
          <template v-if="device.ops.server?.state === 'error'">
            <Notice variant="error" :title="device.ops.server.errorCode ?? 'Failed'">
              {{ device.ops.server.errorMessage }}
            </Notice>
          </template>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Timezone" subtitle="Sent as 15-minute offset">
          <div class="field-row">
            <Field label="Preset" hint="Pick a common zone, or override below.">
              <select
                :value="tzOffset"
                @change="(ev) => (tzOffset = Number((ev.target as HTMLSelectElement).value))"
              >
                <option v-for="t in tzChoices" :key="t.name" :value="t.offset">{{ t.label }}</option>
              </select>
            </Field>
            <Field label="UTC offset (minutes)" :error="tzError ?? undefined">
              <input v-model.number="tzOffset" type="number" step="15" min="-840" max="840" />
              <div class="hint">{{ formatUtcOffset(tzOffset) }}</div>
            </Field>
          </div>
          <div class="row end">
            <Button
              variant="primary"
              :loading="device.ops.timezone?.state === 'running'"
              :disabled="!device.isConnected || !!tzError"
              @click="applyTimezone"
            >
              Save timezone
            </Button>
          </div>
          <template v-if="device.ops.timezone?.state === 'error'">
            <Notice variant="error" :title="device.ops.timezone.errorCode ?? 'Failed'">
              {{ device.ops.timezone.errorMessage }}
            </Notice>
          </template>
        </Frame>
      </div>

      <div class="col-6">
        <Frame title="Local cleanup" subtitle="Acts on the device, not the server">
          <p class="hint">
            Clearing local alarms removes every alarm stored on the Inkpaper itself. Use this before registering
            a new device or after a server reset. Server-side alarms are unaffected until the next sync.
          </p>
          <div class="row end">
            <Button
              variant="danger"
              :loading="device.ops['clear-alarms']?.state === 'running'"
              :disabled="!device.isConnected"
              @click="clearAlarms"
            >
              Clear alarms on device
            </Button>
          </div>
          <template v-if="device.ops['clear-alarms']?.state === 'error'">
            <Notice variant="error" :title="device.ops['clear-alarms'].errorCode ?? 'Failed'">
              {{ device.ops['clear-alarms'].errorMessage }}
            </Notice>
          </template>
        </Frame>
      </div>
    </div>
  </div>
</template>
