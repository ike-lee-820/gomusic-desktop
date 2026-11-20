<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { usePlayerStore } from '../stores/player'
import GoIcon from '../components/GoIcon.vue'
import { paintBackground, renderArcFrame } from '../canvas/RecRenderer'

const router = useRouter()
const player = usePlayerStore()
const bgCanvas = ref<HTMLCanvasElement | null>(null)
const lyricCanvas = ref<HTMLCanvasElement | null>(null)

const song = computed(() => player.current)
const fmt = (ms: number) => {
  const s = Math.max(0, Math.floor(ms / 1000))
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`
}

function repaint() {
  if (bgCanvas.value) paintBackground(bgCanvas.value, song.value?.picUrl ?? null)
  if (lyricCanvas.value) {
    renderArcFrame(lyricCanvas.value, {
      lines: player.lyric.lines.map((l) => l.text),
      focus: Math.min(2, Math.max(0, player.lyric.lines.length - 1)),
      width: lyricCanvas.value.clientWidth,
      height: lyricCanvas.value.clientHeight,
      accent: player.accent,
    })
  }
}

onMounted(() => { repaint(); window.addEventListener('resize', repaint) })
watch([() => song.value?.picUrl, () => player.lyric, () => player.accent], repaint)

function open(path: string) { router.push(path) }
</script>

<template>
  <div class="player">
    <!-- 背景层：单张封面 cover 铺满，只裁边不拉伸；轻微模糊只作用于这一张 -->
    <canvas ref="bgCanvas" class="bg-canvas"></canvas>
    <div class="dim"></div>

    <div class="body">
      <section class="info">
        <div class="cover" @click="song?.source === 'NM' && open('/album-desc/' + song?.albumId)"></div>
        <div class="artist">{{ song?.artist ?? '—' }}</div>
        <div class="name">{{ song?.name ?? '未在播放' }}</div>
        <div class="album">{{ song?.album ?? '' }}</div>
      </section>

      <section class="lyric">
        <canvas ref="lyricCanvas" class="lyric-canvas"></canvas>
        <button v-if="!player.lyric.lines.length" class="no-lyric" @click="repaint">暂无歌词</button>
      </section>
    </div>

    <!-- 控制栏：两行展开态，不自动收起 -->
    <div v-if="player.ctrlOpen" class="control">
      <div class="row no-scrollbar">
        <button title="MV"><GoIcon name="mv" /></button>
        <button title="全屏" @click="open('/about')"><span class="txt">全屏</span></button>
        <button title="均衡器" @click="open('/equalizer')"><GoIcon name="eq" /></button>
        <button title="3D 环绕" @click="open('/surround')"><GoIcon name="surround3d" /></button>
        <button title="录制" @click="open('/recorder')"><GoIcon name="record" /></button>
        <span class="spacer"></span>
        <button title="音质" @click="open('/settings')"><GoIcon name="quality" /><span class="txt">{{ player.level }}</span></button>
        <button title="更多" @click="open('/queue')"><GoIcon name="add" /></button>
        <button class="collapse" title="收起" @click="player.ctrlOpen = false">⟩</button>
      </div>
      <div class="row no-scrollbar">
        <span class="time tabular">{{ fmt(player.positionMs) }}</span>
        <div class="track" @click="player.positionMs = player.durationMs * 0.5">
          <div class="fill" :style="{ width: (player.progress * 100) + '%' }"></div>
          <div class="thumb" :style="{ left: (player.progress * 100) + '%' }"></div>
        </div>
        <span class="time tabular">{{ fmt(player.durationMs) }}</span>
        <button title="上一首" @click="player.positionMs = 0"><GoIcon name="prev" /></button>
        <button :title="player.playing ? '暂停' : '播放'" @click="player.playing = !player.playing">
          <GoIcon :name="player.playing ? 'pause' : 'play'" />
        </button>
        <button title="下一首"><GoIcon name="next" /></button>
        <button title="队列" @click="open('/queue')"><GoIcon name="queue" /></button>
        <button title="循环"><GoIcon name="repeatAll" /></button>
      </div>
    </div>
    <button v-else class="ctrl-collapsed" title="展开控制栏" @click="player.ctrlOpen = true">⟩</button>
  </div>
</template>

<style scoped>
.player { position: relative; width: 100%; height: 100%; overflow: hidden; background: var(--go-bg); }
.bg-canvas { position: absolute; inset: 0; width: 100%; height: 100%; }
.dim { position: absolute; inset: 0; background: var(--go-dim); pointer-events: none; }
.body { position: absolute; inset: 0 0 128px 0; display: flex; }
.info {
  width: clamp(280px, 34%, 460px); padding: 40px;
  display: flex; flex-direction: column; justify-content: center; align-items: flex-start;
}
.cover { width: 96px; height: 96px; border-radius: 12px; background: var(--go-surface-2); cursor: pointer; }
.artist { margin-top: 16px; font-size: 14px; color: var(--go-text-2); }
.name { margin-top: 8px; font-size: 22px; font-weight: 600; line-height: 1.25; }
.album { margin-top: 4px; font-size: 13px; color: var(--go-text-3); }
.lyric { flex: 1; position: relative; min-width: 0; }
.lyric-canvas { width: 100%; height: 100%; }
.no-lyric { position: absolute; inset: 0; margin: auto; color: var(--go-text-2); font-size: 14px; }
.control {
  position: absolute; left: 24px; right: 24px; bottom: 16px; height: 112px;
  background: color-mix(in srgb, var(--go-surface) 92%, transparent);
  backdrop-filter: blur(20px); border: 1px solid var(--go-line);
  border-radius: 16px; padding: 0 24px; display: flex; flex-direction: column;
  justify-content: center; gap: 16px;
}
.row { display: flex; align-items: center; gap: 8px; overflow-x: auto; height: 48px; flex: none; }
.row button { min-width: 44px; height: 44px; display: flex; align-items: center; justify-content: center; gap: 6px; border-radius: 10px; }
.row button:hover { background: var(--go-surface-2); }
.spacer { flex: 1; }
.txt { font-size: 12px; color: var(--go-text-2); }
.collapse { color: var(--go-text-3); }
.time { font-size: 11px; color: var(--go-text-2); min-width: 40px; }
.track { position: relative; flex: 1; height: 4px; background: var(--go-line); border-radius: 2px; cursor: pointer; }
.fill { height: 100%; background: var(--go-accent); border-radius: 2px; }
.thumb { position: absolute; top: -4px; width: 12px; height: 12px; border-radius: 50%; background: var(--go-text); transform: translateX(-50%); }
.ctrl-collapsed { position: absolute; right: 24px; bottom: 16px; width: 32px; height: 64px; border-radius: 10px; background: var(--go-surface); border: 1px solid var(--go-line); color: var(--go-text-2); }
</style>
