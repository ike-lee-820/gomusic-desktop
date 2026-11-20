<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()
const title = computed(() => String(route.meta.title ?? ''))
const note = computed(() => String(route.meta.note ?? ''))
</script>

<template>
  <div class="page">
    <div class="head">
      <button class="back" @click="router.back()">←</button>
      <h1>{{ title }}</h1>
    </div>
    <div class="scroll no-scrollbar">
      <div class="go-card">
        <div class="go-group-title">页面规格</div>
        <p class="note selectable">{{ note }}</p>
      </div>
      <div class="go-card">
        <div class="go-group-title">状态</div>
        <div class="go-row"><span>布局</span><span class="chip">仅横屏 · 76px 导航</span></div>
        <div class="go-row"><span>IPC</span><span class="chip">命令签名已冻结</span></div>
        <div class="go-row"><span>实现</span><span class="chip">基线骨架 · 待接入</span></div>
      </div>
      <div class="tail"></div>
    </div>
  </div>
</template>

<style scoped>
.page { height: 100%; display: flex; flex-direction: column; }
.head { flex: none; height: 56px; display: flex; align-items: center; gap: 12px; padding: 0 24px; }
.back { width: 32px; height: 32px; border-radius: 8px; color: var(--go-text-2); }
.back:hover { background: var(--go-surface-2); }
h1 { margin: 0; font-size: 16px; font-weight: 600; }
.scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 24px 32px; display: flex; flex-direction: column; gap: 16px; }
.note { margin: 0; font-size: 14px; line-height: 1.8; color: var(--go-text-2); }
.chip { font-size: 12px; color: var(--go-accent); background: var(--go-accent-8); padding: 4px 10px; border-radius: var(--r-chip); }
.tail { height: 32px; flex: none; }
</style>
