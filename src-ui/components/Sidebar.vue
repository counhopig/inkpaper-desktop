<script setup lang="ts">
import { computed } from "vue";
import { useDeviceStore } from "../stores/device";
import StatusMark from "./StatusMark.vue";

export type PageId = "overview" | "device" | "content" | "logs";

const props = defineProps<{
  active: PageId;
}>();
const emit = defineEmits<{ (e: "navigate", page: PageId): void }>();

const device = useDeviceStore();

const items: Array<{ id: PageId; label: string; hint: string; glyph: string }> = [
  { id: "overview", label: "Overview", hint: "Status at a glance", glyph: "◉" },
  { id: "device", label: "Device", hint: "Connect and configure", glyph: "▣" },
  { id: "content", label: "Content", hint: "Alarms and todos", glyph: "≡" },
  { id: "logs", label: "Logs", hint: "Diagnostics", glyph: "≋" },
];

const status = computed(() => {
  if (!device.connection.connected) return "idle" as const;
  return "ok" as const;
});
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-brand">
      <div class="mark" aria-hidden="true">Ip</div>
      <div class="wordmark">
        <div class="name">Inkpaper</div>
        <div class="sub">Desktop</div>
      </div>
    </div>

    <nav class="sidebar-nav" aria-label="Primary">
      <button
        v-for="item in items"
        :key="item.id"
        :class="['nav-item', { active: props.active === item.id }]"
        @click="emit('navigate', item.id)"
      >
        <span class="nav-glyph">{{ item.glyph }}</span>
        <span class="nav-text">
          <span class="nav-label">{{ item.label }}</span>
          <span class="nav-hint">{{ item.hint }}</span>
        </span>
        <span v-if="props.active === item.id" class="nav-caret">─</span>
      </button>
    </nav>

    <div class="sidebar-foot">
      <StatusMark :status="status" :label="device.connection.kind" />
      <div class="sidebar-port">{{ device.connection.port }}</div>
    </div>
  </aside>
</template>

<style scoped>
.nav-item {
  display: grid;
  grid-template-columns: 22px 1fr auto;
  align-items: center;
  gap: var(--s-2);
  padding: var(--s-2) var(--s-3);
  background: transparent;
  border: 0;
  border-radius: 0;
  text-align: left;
  cursor: pointer;
  color: var(--ink);
  transition: background var(--t-fast) var(--ease);
}
.nav-item:hover { background: var(--surface-raised); }
.nav-item.active {
  background: var(--surface);
  outline: var(--b-double);
  outline-offset: -3px;
}

.nav-glyph {
  font-family: var(--font-mono);
  text-align: center;
  color: var(--ink-muted);
  font-size: var(--t-14);
}
.nav-item.active .nav-glyph { color: var(--ink); }

.nav-text {
  display: flex;
  flex-direction: column;
  gap: 0;
  min-width: 0;
}
.nav-label {
  font-size: var(--t-13);
  font-weight: 500;
  line-height: 1.2;
}
.nav-hint {
  font-size: var(--t-11);
  color: var(--ink-muted);
  letter-spacing: 0.03em;
}

.nav-caret {
  font-family: var(--font-mono);
  color: var(--ink);
  font-size: var(--t-13);
}

.sidebar-port {
  margin-top: var(--s-1);
  font-family: var(--font-mono);
  font-size: var(--t-11);
  color: var(--ink-muted);
  word-break: break-all;
}

@media (max-width: 760px) {
  .nav-text, .nav-caret { display: none; }
  .sidebar-brand .wordmark { display: none; }
  .sidebar-brand { justify-content: center; padding-left: 0; padding-right: 0; }
  .sidebar-foot {
    padding: var(--s-3) 0 0;
    margin: var(--s-3) var(--s-2) 0;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .sidebar-foot :deep(.mark) { font-size: 0; gap: 0; }
  .sidebar-foot :deep(.mark)::before { font-size: var(--t-15); }
  .sidebar-port { display: none; }
}
</style>
