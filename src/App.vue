<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import AppTitleBar from './components/AppTitleBar.vue'
import NavRail from './components/NavRail.vue'
import UpdateToast from './components/UpdateToast.vue'
import { usePlayerStore } from './stores/player'
import { useUpdateStore } from './stores/update'

const router = useRouter()
const player = usePlayerStore()
const update = useUpdateStore()
const ctrl = ref<HTMLElement | null>(null)

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

onMounted(() => {
  window.addEventListener('keydown', onKey)
  // 应用启动自动检查更新：默认 gh-proxy 加速，403/超时自动回退直连，失败不阻断界面。
  void update.check()
})
onUnmounted(() => window.removeEventListener('keydown', onKey))
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
  </div>
</template>

<style scoped>
.app-shell { width: 100%; height: 100%; display: flex; flex-direction: column; }
.main { flex: 1; min-height: 0; display: flex; }
.content { flex: 1; min-width: 0; overflow: hidden; }
.content-inner { width: 100%; height: 100%; }
/* 宽屏仅增加外侧留白，不新增任何栏 */
@media (min-width: 1600px) {
  .content-inner { max-width: 1440px; margin: 0 auto; }
}
</style>
