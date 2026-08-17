<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    variant?: "default" | "primary";
    spacing?: "default" | "tight" | "loose" | "flush";
  }>(),
  { variant: "default", spacing: "default" },
);
</script>

<template>
  <section :class="['frame', variant === 'primary' && 'primary', spacing === 'tight' && 'tight', spacing === 'loose' && 'loose', spacing === 'flush' && 'flush']">
    <header v-if="title || subtitle || $slots.header" class="frame-title">
      <div>
        <slot name="header">
          <h2>{{ title }}</h2>
        </slot>
      </div>
      <span v-if="subtitle" class="sub">{{ subtitle }}</span>
    </header>
    <slot />
  </section>
</template>
