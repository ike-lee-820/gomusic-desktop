<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'

const route = useRoute()
const router = useRouter()
const title = computed(() => String(route.meta.title ?? 'GoMusic'))

function back() { router.back() }
function forward() { router.forward() }
async function win(action: 'minimize' | 'toggleMaximize' | 'close') {
  try {
    const w = getCurrentWindow()
    if (action === 'minimize') await w.minimize()
    else if (action === 'toggleMaximize') await w.toggleMaximize()
    else await w.close()
  } catch { /* 浏览器预览时忽略 */ }
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="left">
      <button class="nav-btn" title="返回" @click="back">←</button>
      <button class="nav-btn" title="前进" @click="forward">→</button>
      <span class="crumb">{{ title }}</span>
    </div>
    <span class="hint">Ctrl/⌘ + K 搜索</span>
    <div class="right">
      <button class="win-btn" title="最小化" @click="win('minimize')">─</button>
      <button class="win-btn" title="最大化" @click="win('toggleMaximize')">□</button>
      <button class="win-btn win-btn--close" title="关闭" @click="win('close')">×</button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: var(--topbar-h); flex: none; display: flex; align-items: center;
  justify-content: space-between; padding-left: var(--sp-2);
  background: var(--go-surface); border-bottom: 1px solid var(--go-line);
  font-size: 13px; color: var(--go-text-2);
}
.left, .right { display: flex; align-items: center; }
.nav-btn { width: 32px; height: 32px; border-radius: 8px; color: var(--go-text-2); }
.nav-btn:hover { background: var(--go-surface-2); color: var(--go-text); }
.crumb { margin-left: var(--sp-3); }
.hint { font-size: 11px; color: var(--go-text-3); }
.win-btn { width: 40px; height: 40px; font-size: 14px; color: var(--go-text-2); }
.win-btn:hover { background: var(--go-surface-2); color: var(--go-text); }
.win-btn--close:hover { background: var(--go-danger); color: #fff; }
</style>
