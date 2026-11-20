<script setup lang="ts">
withDefaults(defineProps<{ label: string; active?: boolean; disabled?: boolean }>(), {
  active: false, disabled: false,
})
defineEmits<{ (e: 'select'): void }>()
</script>

<template>
  <button class="chip" :class="{ 'is-active': active }" :disabled="disabled" @click="$emit('select')">
    {{ label }}
  </button>
</template>

<style scoped>
.chip {
  height: 28px; padding: 0 12px; border-radius: var(--r-chip);
  border: 1px solid var(--go-line); font-size: 12px; color: var(--go-text-2);
  background: var(--go-surface); white-space: nowrap;
}
.chip:hover:not(:disabled) { background: var(--go-surface-2); color: var(--go-text); }
.chip.is-active { color: var(--go-accent); background: var(--go-accent-8); border-color: transparent; }
.chip:disabled { opacity: .35; cursor: default; }
</style>
