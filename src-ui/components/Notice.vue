<script setup lang="ts">
import { computed } from "vue";

type Variant = "info" | "warn" | "error";

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    title?: string;
    detail?: string;
  }>(),
  { variant: "info" },
);

const glyph = computed(() => {
  switch (props.variant) {
    case "info": return "i";
    case "warn": return "△";
    case "error": return "✕";
  }
});
</script>

<template>
  <div :class="['notice', variant]">
    <span class="glyph">{{ glyph }}</span>
    <div class="body">
      <div v-if="title" class="title">{{ title }}</div>
      <slot />
      <div v-if="detail" class="detail">{{ detail }}</div>
    </div>
  </div>
</template>
