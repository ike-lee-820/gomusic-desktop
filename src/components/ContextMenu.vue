<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import type { MenuItem } from './menu'

const items = ref<MenuItem[]>([])
const x = ref(0), y = ref(0), open = ref(false)

function show(px: number, py: number, list: MenuItem[]) {
  items.value = list
  const w = 200, h = list.length * 36 + 8
  x.value = px + w > window.innerWidth ? Math.max(0, px - w) : px
  y.value = py + h > window.innerHeight ? Math.max(0, py - h) : py
  open.value = true
}
function hide() { open.value = false }
function run(it: MenuItem) { hide(); it.action() }

onMounted(() => window.addEventListener('click', hide))
onUnmounted(() => window.removeEventListener('click', hide))
defineExpose({ show, hide })
</script>

<template>
  <div v-if="open" class="menu" :style="{ left: x + 'px', top: y + 'px' }" @click.stop>
    <button v-for="it in items" :key="it.label" :class="{ danger: it.danger }" @click="run(it)">
      <span v-if="it.icon" class="ic">{{ it.icon }}</span>{{ it.label }}
    </button>
  </div>
</template>

<style scoped>
.menu {
  position: fixed; z-index: 120; width: 200px; padding: 4px;
  background: var(--go-surface-2); border: 1px solid var(--go-line);
  border-radius: var(--r-menu); box-shadow: var(--shadow-float);
}
.menu button {
  display: flex; align-items: center; gap: 8px; width: 100%; height: 36px;
  padding: 0 10px; border-radius: 8px; font-size: 13px; color: var(--go-text); text-align: left;
}
.menu button:hover { background: var(--go-surface); }
.menu button.danger { color: var(--go-danger); }
.ic { width: 16px; color: var(--go-text-3); font-size: 12px; }
</style>
