<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useDeviceStore } from "../stores/device";
import { useServerStore } from "../stores/server";

const device = useDeviceStore();
const server = useServerStore();

const nowMs = ref(Date.now());
let clockTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  clockTimer = setInterval(() => (nowMs.value = Date.now()), 1000);
});
onBeforeUnmount(() => clearInterval(clockTimer));

const now = computed(() =>
  new Date(nowMs.value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }),
);

const date = computed(() =>
  new Date(nowMs.value).toLocaleDateString([], {
    weekday: "short",
    month: "short",
    day: "numeric",
  }),
);

const deviceGlyph = computed(() => {
  if (!device.connection.connected) return "○";
  return "◉";
});

const deviceLabel = computed(() => {
  if (!device.connection.connected) return "Offline";
  return device.connection.kind;
});
</script>

<template>
  <header class="topbar">
    <div class="topbar-status">
      <span class="status-chip" :class="{ on: device.connection.connected }">
        <span class="glyph">{{ deviceGlyph }}</span>
        <span class="label">Device</span>
        <span class="value">{{ deviceLabel }}</span>
      </span>
      <span class="status-chip" :class="{ on: server.connected }">
        <span class="glyph">{{ server.connected ? "◉" : "○" }}</span>
        <span class="label">Server</span>
        <span class="value">{{ server.connected ? "linked" : "—" }}</span>
      </span>
    </div>
    <span class="topbar-clock"><span class="topbar-date">{{ date }}</span>{{ now }}</span>
  </header>
</template>

<style scoped>
.status-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--s-2);
  font-size: var(--t-12);
  color: var(--ink-muted);
  letter-spacing: 0.03em;
}
.status-chip .glyph {
  font-family: var(--font-mono);
  font-size: var(--t-14);
  line-height: 1;
}
.status-chip .label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: var(--t-11);
}
.status-chip .value {
  font-family: var(--font-mono);
  font-size: var(--t-12);
  padding: 1px var(--s-2);
  border: var(--b-1);
  text-transform: lowercase;
}
.status-chip.on .glyph { color: var(--ink); }
.status-chip.on .value { border-width: 2px; font-weight: 600; }
</style>
