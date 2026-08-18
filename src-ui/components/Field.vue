<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  label?: string;
  hint?: string;
  error?: string;
}>();

const invalid = computed(() => !!props.error);
</script>

<template>
  <div :class="['field', { invalid }]">
    <label v-if="label">{{ label }}</label>
    <slot />
    <div v-if="error" class="err">{{ error }}</div>
    <div v-else-if="hint" class="hint">{{ hint }}</div>
  </div>
</template>
