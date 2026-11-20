<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import AppTitleBar from './components/AppTitleBar.vue'
import NavRail from './components/NavRail.vue'
import UpdateToast from './components/UpdateToast.vue'
import Toast from './components/Toast.vue'
import { usePlayerStore } from './stores/player'
import { useUpdateStore } from './stores/update'
import type { UpdateDoc } from './api/types'

const router = useRouter()
const player = usePlayerStore()
const update = useUpdateStore()
const ctrl = ref<HTMLElement | null>(null)
let unlisten: (() => void) | null = null

// §6.2.1 桌面增强：不改变视觉，仅键盘映射
function onKey(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null
  if (target && /INPUT|TEXTAREA/.test(target.tagName)) return
  const mod = e.ctrlKey || e.metaKey
  if (mod && e.key.toLowerCase() === 'k') { e.preventDefault(); router.push('/search') }
  else if (mod && e.key.toLowerCase() === 'e') { e.preventDefault(); router.push('/equalizer') }
  else if (mod && e.key === ',') { e.preventDefault(); router.push('/settings') }
  else if (mod && e.key.toLowerCase() === 'r') { e.preventDefault(); router.push('/recorder') }
  else if (mod && e.key === 'ArrowLeft') { e.preventDefault(); router.back() }
  else if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); router.back() }
  else if (e.key === 'Escape') { router.back() }
  else if (e.key === ' ') { e.preventDefault(); player.playing = !player.playing }
  else if (e.key === 'ArrowLeft') { player.positionMs = Math.max(0, player.positionMs - (e.shiftKey ? 15000 : 5000)) }
  else if (e.key === 'ArrowRight') { player.positionMs += e.shiftKey ? 15000 : 5000 }
}

onMounted(async () => {
  window.addEventListener('keydown', onKey)
  // Rust 侧 App 启动即检查更新（gh-proxy 优先 + 403 回退直连），结果以事件广播。
  try {
    unlisten = await listen<UpdateDoc>('update://available', (e) => {
      if (!update.latest) update.latest = e.payload
      void update.ensureOffline()
    })
  } catch { /* 浏览器预览态 */ }
  // 前端兜底：预览态或 Rust 事件未到达时，启动也自动检查一次。
  window.setTimeout(() => { if (!update.latest) void update.check() }, 1200)
})
onUnmounted(() => { window.removeEventListener('keydown', onKey); unlisten?.() })
</script>

<template>
  <div class="app-shell">
    <AppTitleBar />
    <div class="main">
      <NavRail />
      <main ref="ctrl" class="content">
        <div class="content-inner"><router-view /></div>
      </main>
    </div>
    <UpdateToast />
    <Toast />
  </div>
</template>

<style scoped>
.app-shell { width: 100%; height: 100%; display: flex; flex-direction: column; }
.main { flex: 1; min-height: 0; display: flex; }
.content { flex: 1; min-width: 0; overflow: hidden; }
.content-inner { width: 100%; height: 100%; }
/* 宽屏仅增加外侧留白，不新增任何栏；无竖屏断点 */
@media (min-width: 1600px) {
  .content-inner { max-width: 1440px; margin: 0 auto; }
}
</style>
