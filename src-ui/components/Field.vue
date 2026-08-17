<script setup lang="ts">
import { computed, useAttrs } from "vue";

defineOptions({ inheritAttrs: false });

const props = defineProps<{
  label?: string;
  hint?: string;
  error?: string;
}>();

const attrs = useAttrs();
const invalid = computed(() => !!props.error);
const inputAttrs = computed(() => {
  const { class: _c, ...rest } = attrs as Record<string, unknown>;
  return rest;
});
</script>

<template>
  <div :class="['field', { invalid }]">
    <label v-if="label">{{ label }}</label>
    <!-- Pass-through: parent provides the actual input/select/textarea via default slot. -->
    <slot v-bind="inputAttrs" />
    <div v-if="error" class="err">{{ error }}</div>
    <div v-else-if="hint" class="hint">{{ hint }}</div>
  </div>
</template>
