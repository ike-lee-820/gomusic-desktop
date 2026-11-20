<script setup lang="ts">
import { useRouter } from 'vue-router'

defineProps<{ title: string }>()
const router = useRouter()
</script>

<template>
  <div class="page">
    <div class="head">
      <button class="back" title="返回" @click="router.back()">←</button>
      <h1>{{ title }}</h1>
      <div class="ops"><slot name="ops" /></div>
    </div>
    <div class="scroll no-scrollbar">
      <slot />
      <div class="tail"></div>
    </div>
  </div>
</template>

<style scoped>
.page { height: 100%; display: flex; flex-direction: column; }
.head { flex: none; height: 56px; display: flex; align-items: center; gap: 12px; padding: 0 24px; }
.back { width: 32px; height: 32px; border-radius: 8px; color: var(--go-text-2); flex: none; }
.back:hover { background: var(--go-surface-2); }
h1 { margin: 0; font-size: 16px; font-weight: 600; flex: none; }
.ops { margin-left: auto; display: flex; align-items: center; gap: 8px; }
.ops :deep(button) { font-size: 13px; color: var(--go-accent); padding: 6px 10px; border-radius: 8px; }
.ops :deep(button:hover) { background: var(--go-accent-8); }
.scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 24px 32px; display: flex; flex-direction: column; gap: 16px; }
.tail { height: 32px; flex: none; }
</style>
