<script setup lang="ts">
import { computed } from 'vue'
import { PLAYER_ICONS } from '../assets/icons'

const props = withDefaults(defineProps<{ name: string; size?: number; active?: boolean }>(), {
  size: 24,
  active: false,
})

const uri = computed(() => (PLAYER_ICONS as Record<string, string>)[props.name] ?? '')
const style = computed(() => ({
  width: `${props.size}px`,
  height: `${props.size}px`,
  '--icon': uri.value ? `url("${uri.value}")` : 'none',
  color: props.active ? 'var(--go-accent)' : 'var(--go-text)',
}))
</script>

<template>
  <span v-if="uri" class="go-icon" :style="style" aria-hidden="true" />
  <span v-else class="go-icon go-icon--fallback" :style="style" aria-hidden="true" />
</template>

<style scoped>
.go-icon {
  display: inline-block; flex: none;
  background: currentColor;
  -webkit-mask: var(--icon) center / contain no-repeat;
  mask: var(--icon) center / contain no-repeat;
}
.go-icon--fallback {
  border: 1.7px solid currentColor;
  border-radius: 5px;
  opacity: 0.45;
}
.go-icon.is-disabled { opacity: 0.35; }
</style>
