<script setup lang="ts">
import { ref } from 'vue'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import SliderCard from '../components/SliderCard.vue'
import { ipc } from '../api/ipc'
import type { SurroundConfig } from '../api/types'
import { toast } from '../composables/overlay'

const enabled = ref(false), strength = ref(50), scene = ref('耳机')
const advanced = ref(false), width = ref(1.2), distance = ref(1.0), reverb = ref(0.3), eco = ref(false)
const taps = [7, 11, 19, 29, 37]
const ab = ref(false)

function apply() {
  const c: SurroundConfig = { enabled: enabled.value, strength: strength.value, scene: scene.value, width: width.value, distance: distance.value, taps, reverb: reverb.value, eco: eco.value }
  void ipc.surroundApply(c).catch(() => {})
  toast.show('已应用（30ms 淡变）')
}
function reset() { strength.value = 50; width.value = 1.2; distance.value = 1; reverb.value = .3; eco.value = false; apply() }
</script>

<template>
  <PageScaffold title="3D 环绕">
    <template #ops><button @click="ab = !ab">A·B</button><button @click="reset">重置</button><button @click="reset">恢复默认</button></template>

    <div class="go-card">
      <div class="go-group-title">默认层</div>
      <div class="go-row"><span>总开关</span><button class="sw" :class="{ on: enabled }" @click="enabled = !enabled; apply()">{{ enabled ? '开启' : '关闭' }}</button></div>
      <div class="go-row"><span>场景</span><div class="chips">
        <Chip v-for="s in ['耳机', '音箱', '车载']" :key="s" :label="s" :active="scene === s" @select="scene = s; apply()" />
      </div></div>
    </div>

    <SliderCard v-model="strength" title="强度" :min="0" :max="100" @update:model-value="apply" />

    <div class="go-card">
      <div class="go-group-title clickable" @click="advanced = !advanced">高级层 {{ advanced ? '▾' : '⟩' }}</div>
      <template v-if="advanced">
        <SliderCard v-model="width" title="声场宽度" :min="0.5" :max="2" :step="0.1" unit="" @update:model-value="apply" />
        <SliderCard v-model="distance" title="距离" :min="0.5" :max="3" :step="0.1" unit="" @update:model-value="apply" />
        <div class="go-row"><span>反射抽头</span><span class="tabular">{{ taps.join(' / ') }} ms</span></div>
        <SliderCard v-model="reverb" title="混响时间" :min="0" :max="1" :step="0.05" unit="s" @update:model-value="apply" />
        <div class="go-row"><span>省电档</span><button class="act" @click="eco = !eco; apply()">{{ eco ? '开' : '关' }}</button></div>
      </template>
    </div>

    <p class="note">独立 DSP，不使用均衡器；150Hz 以下不空间化。交叉馈送 + ITD≈0.25ms + 高频滚降，输出软限幅 −1dBFS。</p>
  </PageScaffold>
</template>

<style scoped>
.chips { display: flex; gap: 8px; }
.sw { font-size: 12px; color: var(--go-text-3); border: 1px solid var(--go-line); border-radius: var(--r-chip); padding: 4px 12px; }
.sw.on { color: var(--go-accent); background: var(--go-accent-8); border-color: transparent; }
.clickable { cursor: pointer; }
.act { font-size: 12px; color: var(--go-accent); }
.note { font-size: 13px; color: var(--go-text-3); line-height: 1.8; margin: 0; }
</style>
