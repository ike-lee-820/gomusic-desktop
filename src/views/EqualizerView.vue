<script setup lang="ts">
import { computed, ref } from 'vue'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import SliderCard from '../components/SliderCard.vue'
import { ipc } from '../api/ipc'
import type { EqConfig } from '../api/types'
import { toast } from '../composables/overlay'

const groups = [
  { name: '人声', items: [{ n: '纯净人声', d: '人声更靠前、齿音不刺耳' }] },
  { name: '音乐', items: [{ n: '澎湃乐场', d: '低频与声场同时抬升' }, { n: '沉重低音', d: '强调 60–120Hz' }, { n: '温暖圆润', d: '柔化高频毛刺' }, { n: '冷冽明亮', d: '提升 8k 以上空气感' }] },
  { name: '空间', items: [{ n: '缥缈音场', d: '轻量早期反射' }, { n: '现场环绕', d: '更宽的舞台感' }] },
  { name: '创意', items: [{ n: '复古音效（电话 / 老式收音机）', d: '默认关闭，切换前 30ms 淡变' }] },
  { name: '夜间', items: [{ n: '深夜聆听', d: '动态等响度，非固定预设' }] },
]
const presets = groups.flatMap((g) => g.items.map((i) => ({ ...i, group: g.name })))
const freq = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]
const bands = ref(freq.map((f) => ({ freq: f, gain: 0, q: 1 })))
const strength = ref(70), hpf = ref(20), lpf = ref(20000), q = ref(1)
const enabled = ref(new Set<string>(['纯净人声']))
const advanced = ref(false), ab = ref(false)
const device = ref('耳机')

const warn = computed(() => {
  const low = bands.value.filter((b) => b.freq <= 250).some((b) => b.gain > 4.5)
  const mid = bands.value.filter((b) => b.freq > 250 && b.freq <= 4000).some((b) => b.gain > 6)
  const high = bands.value.filter((b) => b.freq > 4000).some((b) => b.gain > 3)
  return low || mid || high
})

function config(): EqConfig {
  return { preset: [...enabled.value][0] ?? 'flat', strength: strength.value, bands: bands.value.map((b) => ({ ...b, q: q.value })), hpf: hpf.value, lpf: lpf.value, deviceCalib: device.value, abLoudness: ab.value }
}
function apply() { void ipc.eqApply(config()).catch(() => {}); toast.show('已应用（30ms 淡变）') }
function toggle(n: string) {
  const s = new Set(enabled.value)
  if (s.has(n)) s.delete(n); else { s.clear(); s.add(n) }
  enabled.value = s; apply()
}
function reset() { bands.value = freq.map((f) => ({ freq: f, gain: 0, q: 1 })); strength.value = 70; q.value = 1; hpf.value = 20; lpf.value = 20000; apply() }
</script>

<template>
  <PageScaffold title="均衡器">
    <template #ops>
      <button @click="ab = !ab">A·B 对比</button><button @click="reset">重置</button><button @click="reset">恢复默认</button>
    </template>

    <div class="go-card">
      <div class="go-group-title">预设 · 共 9 预设 + 老式收音机变体</div>
      <div v-for="g in groups" :key="g.name" class="group">
        <div class="gname">{{ g.name }}<span v-if="g.name === '创意'"> · 默认关闭</span></div>
        <div class="preset-grid">
          <button v-for="p in g.items" :key="p.n" class="preset" :class="{ on: enabled.has(p.n) }" @click="toggle(p.n)">
            <span class="pn">{{ p.n }}</span><span class="pd">{{ p.d }}</span><span class="sw"></span>
          </button>
        </div>
      </div>
    </div>

    <SliderCard v-model="strength" title="强度" :min="0" :max="100" :step="1" @update:model-value="apply" />

    <div class="go-card">
      <div class="go-group-title clickable" @click="advanced = !advanced">高级 · 10 段推子 {{ advanced ? '▾' : '⟩' }}</div>
      <div v-if="advanced" class="eq">
        <div v-for="(b, i) in bands" :key="b.freq" class="band">
          <span class="bv tabular">{{ b.gain > 0 ? '+' : '' }}{{ b.gain.toFixed(1) }}</span>
          <input v-model.number="bands[i].gain" type="range" min="-12" max="12" step="0.5" class="vertical" @change="apply" />
          <span class="bf tabular">{{ b.freq >= 1000 ? (b.freq / 1000) + 'k' : b.freq }}</span>
        </div>
      </div>
      <div v-if="advanced" class="adv">
        <div class="go-row"><span>Q 值</span><input v-model.number="q" type="range" min="0.3" max="8" step="0.1" @change="apply" /></div>
        <div class="go-row"><span>HPF</span><span class="tabular">{{ hpf }} Hz</span></div>
        <div class="go-row"><span>LPF</span><span class="tabular">{{ lpf / 1000 }} kHz</span></div>
        <div v-if="warn" class="warn">⚠ 低频 &gt; 4.5dB / 中频 &gt; 6dB / 高频 &gt; 3dB，请注意削波</div>
      </div>
    </div>

    <div class="go-card">
      <div class="go-group-title">设备校准</div>
      <div class="chips"><Chip v-for="d in ['耳机', '音箱', '车载']" :key="d" :label="d" :active="device === d" @select="device = d; apply()" /></div>
    </div>
  </PageScaffold>
</template>

<style scoped>
.group { margin-bottom: 16px; }
.gname { font-size: 11px; font-weight: 600; letter-spacing: .08em; color: var(--go-text-3); margin-bottom: 8px; }
.preset-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.preset { position: relative; height: 88px; text-align: left; padding: 14px; border: 1px solid var(--go-line); border-radius: 12px; background: var(--go-surface-2); }
.preset.on { border-color: var(--go-accent); background: var(--go-accent-8); }
.pn { display: block; font-size: 14px; font-weight: 600; }
.pd { display: block; font-size: 11px; color: var(--go-text-2); margin-top: 6px; }
.sw { position: absolute; right: 12px; bottom: 12px; width: 24px; height: 12px; border-radius: 999px; background: var(--go-line); }
.preset.on .sw { background: var(--go-accent); }
.clickable { cursor: pointer; }
.eq { display: flex; justify-content: space-between; gap: 8px; margin: 16px 0; }
.band { display: flex; flex-direction: column; align-items: center; gap: 6px; flex: 1; }
.bv, .bf { font-size: 10px; color: var(--go-text-3); }
.vertical { writing-mode: bt-lr; appearance: slider-vertical; width: 4px; height: 160px; accent-color: var(--go-accent); }
.adv { margin-top: 12px; }
.adv input[type='range'] { width: 200px; accent-color: var(--go-accent); }
.warn { margin-top: 12px; font-size: 12px; color: var(--go-warn); }
.chips { display: flex; gap: 8px; }
</style>
