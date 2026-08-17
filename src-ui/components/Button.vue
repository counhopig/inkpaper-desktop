<script setup lang="ts">
import { computed } from "vue";

type Variant = "default" | "primary" | "ghost" | "danger";
type Size = "default" | "small";

const props = withDefaults(
  defineProps<{
    variant?: Variant;
    size?: Size;
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  {
    variant: "default",
    size: "default",
    type: "button",
    disabled: false,
    loading: false,
  },
);

defineEmits<{ (e: "click", ev: MouseEvent): void }>();

const cls = computed(() => {
  const out: string[] = ["btn"];
  if (props.variant === "primary") out.push("primary");
  if (props.variant === "ghost") out.push("ghost");
  if (props.variant === "danger") out.push("danger");
  if (props.size === "small") out.push("small");
  return out.join(" ");
});
</script>

<template>
  <button
    :class="cls"
    :type="type"
    :disabled="disabled || loading"
    @click="(ev) => $emit('click', ev)"
  >
    <span v-if="loading" class="dots" aria-hidden="true"><span /><span /><span /></span>
    <slot />
  </button>
</template>
