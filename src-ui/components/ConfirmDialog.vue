<script setup lang="ts">
import Button from "./Button.vue";

defineProps<{
  open: boolean;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}>();

const emit = defineEmits<{
  (e: "confirm"): void;
  (e: "cancel"): void;
}>();
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="dialog-mask" @click.self="$emit('cancel')">
      <div class="dialog" role="dialog" aria-modal="true">
        <h3>{{ title }}</h3>
        <p v-if="description" class="desc">{{ description }}</p>
        <slot />
        <div class="actions">
          <Button variant="ghost" @click="$emit('cancel')">
            {{ cancelLabel ?? "Cancel" }}
          </Button>
          <Button
            :variant="destructive ? 'danger' : 'primary'"
            @click="$emit('confirm')"
          >
            {{ confirmLabel ?? "Confirm" }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
