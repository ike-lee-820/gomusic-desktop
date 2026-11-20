<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import SliderCard from '../components/SliderCard.vue'
import ProgressBar from '../components/ProgressBar.vue'
import { ipc, progressChannel } from '../api/ipc'
import { toast } from '../composables/overlay'
import { usePlayerStore } from '../stores/player'
import { useRecorderStore } from '../stores/recorder'
import { renderFrame, framePixels, frameStats } from '../canvas/RecRenderer'

const router = useRouter()
const player = usePlayerStore()
const rec = useRecorderStore()
const canvas = ref<HTMLCanvasElement | null>(null)
const levels = ref<string[]>([])
const width = ref(960), height = ref(540)

const ratios = ['16:9', '9:16', '1:1', '4:3']
const styles = [{ k: 'CLASSIC', n: '经典' }, { k: 'DISC', n: '圆盘' }]
const bgs = [{ k: 'GAUSS', n: '高斯模糊' }, { k: 'RAIN', n: '雨季' }, { k: 'SNOW', n: '雪天' }]
const vocals = [{ k: 'ORIGINAL', n: '原声' }, { k: 'REDUCE', n: '减弱' }]

function ratioSize() {
  const [a, b] = rec.config.ratio.split(':').map(Number)
  const h = rec.config.resolution
  height.value = h
  width.value = Math.round((h * a) / b)
  if (canvas.value) { canvas.value.width = width.value; canvas.value.height = height.value }
}

/** 页面上的 canvas 就是 RecRenderer 输出，录制时逐帧把同一张 canvas 的像素推给 Rust。 */
function repaint() {
  if (!canvas.value) return
  const song = player.current
  const durationMs = song?.durationMs || 240000
  renderFrame(canvas.value, {
    width: width.value, height: height.value, song,
    lines: player.lyric.lines.map((l) => l.text),
    focus: Math.min(2, Math.max(0, player.lyric.lines.length - 1)),
    config: rec.config,
    positionMs: Math.floor(durationMs / 2), durationMs,
    accent: player.accent, cover: null,
  })
  canvas.value.dataset.bg = rec.config.background.toLowerCase()
}

let debounce: number | null = null
watch(() => ({ ...rec.config }), () => {
  if (debounce) window.clearTimeout(debounce)
  debounce = window.setTimeout(repaint, 120) // 配置防抖 120ms
}, { deep: true })

async function probe() {
  try {
    levels.value = await ipc.recProbeLevels(player.current?.id ?? 0)
    rec.setLevels(levels.value)
  } catch {
    levels.value = ['higher', 'lossless', 'exhigh']
    rec.setLevels(levels.value)
  }
}

async function start() {
  if (rec.running) return
  rec.running = true
  rec.reset()
  rec.running = true
  try {
    const ch = progressChannel((p) => {
      rec.onProgress(p)
      // 同一张 canvas 的像素推给编码端，不存在第二套预览布局。
      if (canvas.value && p.phase === 3) {
        void ipc.recPushFrame(framePixels(canvas.value), Math.round((p.frame / Math.max(1, p.total)) * 1000)).catch(() => {})
      }
    })
    await ipc.recStart({ ...rec.config }, ch)
    rec.running = false
    const s = frameStats()
    toast.show(`生成完成 · 渲染 ${s.rendered} 帧，复用 ${s.reused} 帧`)
    await loadTasks()
  } catch (e) {
    rec.running = false
    toast.error('生成失败，请重试')
  }
}

async function cancel() { await ipc.recCancel('current').catch(() => {}); rec.reset(); toast.show('已取消') }
async function loadTasks() { try { rec.tasks = await ipc.recList() } catch { /* 离线 */ } }

onMounted(() => { ratioSize(); repaint(); void probe(); void loadTasks() })
onUnmounted(() => { if (debounce) window.clearTimeout(debounce) })
</script>

<template>
  <PageScaffold title="录制视频">
    <template #ops><button @click="start">{{ rec.running ? '生成中…' : '开始生成' }}</button></template>
    <div class="rec">
      <section class="preview">
        <div class="frame-wrap">
          <canvas ref="canvas" class="frame" :width="width" :height="height"></canvas>
        </div>
        <p class="hint">预览固定 = 歌曲时长一半那一帧 · 与成品同源（RecRenderer.ts）</p>
      </section>

      <aside class="config no-scrollbar">
        <div class="go-card">
          <div class="go-group-title">① 可用音质（仅探测通过项）</div>
          <div class="chips">
            <Chip v-for="l in levels" :key="l" :label="l" :active="rec.config.level === l"
                  @select="rec.config.level = l" />
            <span v-if="!levels.length" class="sub">探测中…</span>
          </div>
        </div>

        <div class="go-card">
          <div class="go-group-title">② 画面</div>
          <div class="go-row"><span>比例</span><div class="chips">
            <Chip v-for="r in ratios" :key="r" :label="r" :active="rec.config.ratio === r"
                  @select="rec.config.ratio = r; ratioSize(); repaint()" /></div></div>
          <div class="go-row"><span>分辨率</span><select v-model.number="rec.config.resolution" @change="ratioSize(); repaint()">
            <option v-for="r in [480, 720, 1080, 1440]" :key="r" :value="r">{{ r }}p</option></select></div>
          <div class="go-row"><span>帧率</span><select v-model.number="rec.config.fps">
            <option v-for="f in [24, 30, 60]" :key="f" :value="f">{{ f }} fps</option></select></div>
          <div class="go-row"><span>码率</span><span class="val tabular">{{ rec.config.bitrate }} kbps</span></div>
        </div>

        <div class="go-card">
          <div class="go-group-title">③ 文字</div>
          <SliderCard v-model="rec.config.lyricSize" title="歌词字号" :min="8" :max="96" unit="px" />
          <SliderCard v-model="rec.config.infoSize" title="信息字号" :min="10" :max="64" unit="px" />
          <div class="go-row"><span>译文</span><button class="act" @click="rec.config.showTrans = !rec.config.showTrans">{{ rec.config.showTrans ? '开' : '关' }}</button></div>
          <div class="go-row"><span>字体</span><span class="sub">{{ rec.config.fontSource }}</span></div>
          <div class="go-row"><span>歌词偏移</span><span class="val tabular">{{ rec.config.lyricOffsetMs }} ms</span></div>
          <div class="go-row"><span>粘贴 LRC</span><button class="act" @click="toast.show('可粘贴自定义 LRC，重置恢复原词')">编辑</button></div>
        </div>

        <div class="go-card">
          <div class="go-group-title">④ 样式 / ⑤ 背景 / ⑥ 人声</div>
          <div class="go-row"><span>样式</span><div class="chips">
            <Chip v-for="s in styles" :key="s.k" :label="s.n" :active="rec.config.style === s.k" @select="rec.config.style = s.k as any" /></div></div>
          <div class="go-row"><span>背景</span><div class="chips">
            <Chip v-for="b in bgs" :key="b.k" :label="b.n" :active="rec.config.background === b.k" @select="rec.config.background = b.k as any" /></div></div>
          <div class="go-row"><span>人声</span><div class="chips">
            <Chip v-for="v in vocals" :key="v.k" :label="v.n" :active="rec.config.vocal === v.k" @select="rec.config.vocal = v.k as any" /></div></div>
          <SliderCard v-model="rec.config.reduceStrength" title="减弱强度" :min="0" :max="100" />
        </div>

        <div v-if="rec.progress" class="go-card">
          <div class="go-group-title">生成进度（页面 / 托盘通知 / 录制列表三处同步）</div>
          <div v-for="(name, i) in ['下载音频+歌词', '人声处理', '逐帧录制视频']" :key="name" class="phase">
            <div class="phase-head"><span>{{ i + 1 }} {{ name }}</span><span class="tabular">{{ rec.progress.phase === i + 1 ? rec.progress.percent : (rec.progress.phase > i + 1 ? 100 : 0) }}%</span></div>
            <ProgressBar :value="rec.progress.phase === i + 1 ? rec.progress.percent : (rec.progress.phase > i + 1 ? 100 : 0)" />
            <div v-if="rec.progress.phase === i + 1" class="detail tabular">{{ rec.progress.detail }} · 帧 {{ rec.progress.frame }}/{{ rec.progress.total }}</div>
          </div>
          <button class="cancel" @click="cancel">取消</button>
        </div>
        <div class="tail"></div>
      </aside>
    </div>
  </PageScaffold>
</template>

<style scoped>
.rec { display: flex; gap: 24px; height: 100%; min-height: 0; }
.preview { flex: 1; min-width: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px; }
.frame-wrap { max-height: 34%; max-width: 100%; border: 1px solid var(--go-line); border-radius: 12px; overflow: hidden; background: var(--go-bg); }
.frame { display: block; max-width: 100%; max-height: 240px; width: auto; height: auto; border-radius: 12px; }
.hint { margin: 0; font-size: 12px; color: var(--go-text-3); }
.config { width: 420px; flex: none; overflow-y: auto; display: flex; flex-direction: column; gap: 16px; padding-bottom: 32px; }
.chips { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; max-width: 300px; }
.sub { font-size: 12px; color: var(--go-text-3); }
.val { font-size: 12px; color: var(--go-text-2); }
select { background: var(--go-surface-2); color: var(--go-text); border: 1px solid var(--go-line); border-radius: 8px; padding: 5px 8px; font-size: 12px; }
.act { font-size: 12px; color: var(--go-accent); }
.phase { margin: 10px 0; }
.phase-head { display: flex; justify-content: space-between; font-size: 12px; color: var(--go-text-2); margin-bottom: 6px; }
.detail { font-size: 11px; color: var(--go-text-3); margin-top: 6px; }
.cancel { margin-top: 12px; font-size: 13px; color: var(--go-danger); }
.tail { height: 32px; flex: none; }
</style>
