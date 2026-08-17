<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import Frame from "../components/Frame.vue";
import Button from "../components/Button.vue";
import { useLogsStore, type LogFilter } from "../stores/logs";
import { formatTimeWithDate } from "../lib/format";
import { openLogFolder, exportLog } from "../lib/commands";

const logs = useLogsStore();
const scroller = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);
const exportPath = ref<string | null>(null);

const filterOptions: { id: LogFilter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "device", label: "Device" },
  { id: "server", label: "Server" },
  { id: "error", label: "Error" },
];

const summaryText = computed(() => {
  const s = logs.summary;
  return `${s.info} info · ${s.warn} warn · ${s.error} error`;
});

function onScroll() {
  if (!scroller.value) return;
  const { scrollTop, scrollHeight, clientHeight } = scroller.value;
  stickToBottom.value = scrollHeight - (scrollTop + clientHeight) < 24;
}

async function scrollToBottom() {
  await nextTick();
  if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight;
}

watch(
  () => logs.entries.length,
  () => {
    if (stickToBottom.value) scrollToBottom();
  },
);

onMounted(() => {
  scrollToBottom();
});
onBeforeUnmount(() => {
  logs.flush();
});

function clearView() {
  logs.clear();
}

async function doExportLog() {
  const r = await exportLog();
  if (r.ok) exportPath.value = r.value;
}

async function doOpenFolder() {
  await openLogFolder();
}

function formatLine(e: { timestampMs: number; level: string; source: string; message: string }) {
  return `[${formatTimeWithDate(e.timestampMs)}] ${e.level.toUpperCase()} ${e.source}: ${e.message}`;
}

function copySelected() {
  const text = logs.visibleEntries.map(formatLine).join("\n");
  navigator.clipboard.writeText(text).catch(() => {});
}

function copyAll() {
  const text = logs.entries.map(formatLine).join("\n");
  navigator.clipboard.writeText(text).catch(() => {});
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div>
        <h1 class="page-title">Logs</h1>
        <p class="page-subtitle">Live diagnostics · also written to <span style="font-family: var(--font-mono);">{{ logs.filePath || "(not yet created)" }}</span></p>
      </div>
    </header>

    <Frame spacing="flush">
      <template #header>
        <div class="logs-toolbar">
          <div class="logs-toolbar-row">
            <div class="btn-row">
              <Button
                v-for="f in filterOptions"
                :key="f.id"
                size="small"
                :variant="logs.filter === f.id ? 'primary' : 'default'"
                @click="logs.filter = f.id"
              >
                {{ f.label }}
              </Button>
            </div>
            <div class="logs-search field">
              <input
                :value="logs.query"
                @input="(e) => (logs.query = (e.target as HTMLInputElement).value)"
                type="search"
                placeholder="Search source or message"
              />
            </div>
            <Button
              size="small"
              :variant="logs.paused ? 'primary' : 'default'"
              @click="logs.paused = !logs.paused"
            >
              {{ logs.paused ? "Resume" : "Pause" }}
            </Button>
          </div>
          <div class="logs-toolbar-row">
            <div class="btn-row">
              <Button size="small" variant="ghost" @click="copySelected">Copy view</Button>
              <Button size="small" variant="ghost" @click="copyAll">Copy all</Button>
              <Button size="small" variant="ghost" @click="clearView">Clear view</Button>
              <Button size="small" variant="ghost" @click="doOpenFolder">Open folder</Button>
              <Button size="small" variant="ghost" @click="doExportLog">Export log</Button>
            </div>
          </div>
        </div>
      </template>

      <div v-if="exportPath" class="notice" style="margin: var(--s-2) var(--s-4);">
        <span class="glyph">✓</span>
        <div class="body">Exported to <span style="font-family: var(--font-mono);">{{ exportPath }}</span></div>
      </div>

      <div class="row between" style="padding: 0 var(--s-4); font-size: var(--t-11); color: var(--ink-muted); text-transform: uppercase; letter-spacing: 0.05em; padding-top: var(--s-2);">
        <span>{{ summaryText }}</span>
        <span>{{ logs.visibleEntries.length }} shown</span>
      </div>

      <div
        ref="scroller"
        @scroll="onScroll"
        style="max-height: 60vh; overflow: auto; border-top: var(--b-1-faint); padding: var(--s-2) 0;"
      >
        <div v-if="logs.visibleEntries.length === 0" class="empty-state">
          <div class="glyph">·</div>
          <div class="title">No log entries match this filter</div>
          <div class="sub">Try the “All” filter or clear the search.</div>
        </div>
        <div v-for="e in logs.visibleEntries" :key="e.timestampMs + '-' + e.message" :class="['log-entry', e.level]">
          <span class="ts">{{ formatTimeWithDate(e.timestampMs) }}</span>
          <span class="src">{{ e.source }}</span>
          <span class="lvl">{{ e.level }}</span>
          <span class="msg">{{ e.message }}</span>
        </div>
      </div>
    </Frame>
  </div>
</template>

<style scoped>
.logs-toolbar {
  display: flex;
  flex-direction: column;
}
.logs-toolbar-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--s-3);
  padding: var(--s-3) var(--s-4);
}
.logs-toolbar-row + .logs-toolbar-row {
  padding-top: 0;
}
.logs-search {
  flex: 1 1 220px;
  min-width: 180px;
}
</style>
