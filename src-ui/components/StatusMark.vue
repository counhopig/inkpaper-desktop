<script setup lang="ts">
import { computed } from "vue";

type Status = "idle" | "pending" | "ok" | "warn" | "fail";

const props = defineProps<{
  status: Status;
  label?: string;
}>();

const text = computed(() => props.label ?? defaultLabel(props.status));

function defaultLabel(s: Status): string {
  switch (s) {
    case "idle": return "Offline";
    case "pending": return "Pending";
    case "ok": return "Connected";
    case "warn": return "Attention";
    case "fail": return "Failed";
  }
}
</script>

<template>
  <span :class="['mark', status]">{{ text }}</span>
</template>
