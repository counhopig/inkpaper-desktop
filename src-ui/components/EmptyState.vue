<script setup lang="ts">
defineProps<{
  glyph?: string;
  title?: string;
}>();
</script>

<template>
  <div class="empty-state">
    <div v-if="glyph" class="glyph">{{ glyph }}</div>
    <div v-if="title" class="title">{{ title }}</div>
    <div v-if="$slots.default" class="sub"><slot /></div>
  </div>
</template>
