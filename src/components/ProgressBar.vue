<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{ value: number; total?: number; interactive?: boolean }>(), {
  total: 100, interactive: false,
})
const emit = defineEmits<{ (e: 'seek', ratio: number): void }>()
const ratio = computed(() => props.total > 0 ? Math.min(1, Math.max(0, props.value / props.total)) : 0)

function onClick(e: MouseEvent) {
  if (!props.interactive) return
  const el = e.currentTarget as HTMLElement
  emit('seek', (e.clientX - el.getBoundingClientRect().left) / el.clientWidth)
}
</script>

<template>
  <div class="track" :class="{ interactive }" @click="onClick">
    <div class="fill" :style="{ width: (ratio * 100) + '%' }"></div>
    <div v-if="interactive" class="thumb" :style="{ left: (ratio * 100) + '%' }"></div>
  </div>
</template>

<style scoped>
.track { position: relative; flex: 1; height: 4px; background: var(--go-line); border-radius: 2px; }
.track.interactive { cursor: pointer; }
.fill { height: 100%; background: var(--go-accent); border-radius: 2px; }
.thumb { position: absolute; top: -4px; width: 12px; height: 12px; border-radius: 50%; background: var(--go-text); transform: translateX(-50%); }
</style>
