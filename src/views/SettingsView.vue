<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import SliderCard from '../components/SliderCard.vue'
import { ipc } from '../api/ipc'
import { toast } from '../composables/overlay'
import { useUpdateStore } from '../stores/update'
import { APP_VERSION, BUILD_NUMBER } from '../version'
import { NM_LEVELS } from '../constants'

const router = useRouter()
const update = useUpdateStore()
const levels = NM_LEVELS
const quality = ref('higher')
const font = ref('SOURCE_HAN_SANS')
const wght = ref(400)
const assoc = ref<string[]>([])
const checking = ref(false)

const fonts = [
  { key: 'SYSTEM', label: '系统默认' },
  { key: 'SOURCE_HAN_SERIF', label: '思源宋体' },
  { key: 'SOURCE_HAN_SANS', label: '思源黑体' },
  { key: 'IMPORT', label: '导入' },
]

async function check() {
  checking.value = true
  await update.check(true)
  checking.value = false
  toast.show(update.hasUpdate ? `发现新版本 ${update.latest?.version}` : (update.error || '已是最新版本'))
}
function applyFont() {
  document.documentElement.style.setProperty('--go-wght', String(wght.value))
  void ipc.settingsSet({ fontSource: font.value, wght: wght.value, qualityLevel: quality.value }).catch(() => {})
}
onMounted(async () => {
  try { assoc.value = await ipc.fileAssocStatus() } catch { /* 浏览器预览 */ }
  try {
    const s = await ipc.settingsGet() as Record<string, unknown>
    if (typeof s.qualityLevel === 'string') quality.value = s.qualityLevel
    if (typeof s.wght === 'number') wght.value = s.wght
  } catch { /* 离线 */ }
})
</script>

<template>
  <PageScaffold title="设置">
    <div class="go-card">
      <div class="go-group-title">播放</div>
      <div class="go-row"><span>后台保活（托盘常驻）</span><span class="on">已开启</span></div>
      <div class="go-row">
        <span>默认音质档位</span>
        <select v-model="quality" @change="applyFont">
          <option v-for="l in levels" :key="l" :value="l">{{ l }}</option>
        </select>
      </div>
      <div class="go-row"><span>自动降档策略</span><span class="on">相邻回退 · 风控即停</span></div>
    </div>

    <div class="go-card">
      <div class="go-group-title">网络</div>
      <div class="go-row"><span>更新加速</span><span class="on">gh-proxy.com → 直连兜底</span></div>
      <div class="go-row"><span>风控登录 / 验证</span><button class="act" @click="router.push('/login')">去验证</button></div>
      <div class="go-row"><span>查看日志</span><button class="act" @click="toast.show('日志位于应用数据目录')">打开</button></div>
    </div>

    <div class="go-card">
      <div class="go-group-title">外观</div>
      <div class="go-row">
        <span>字体</span>
        <div class="chips">
          <Chip v-for="f in fonts" :key="f.key" :label="f.label" :active="font === f.key" @select="font = f.key; applyFont()" />
        </div>
      </div>
    </div>
    <SliderCard v-model="wght" title="字重" :min="100" :max="900" :step="10" unit="" @update:model-value="applyFont">
      <div class="sample" :style="{ fontWeight: wght }">GoMusic · 思源可变字重实时样例</div>
    </SliderCard>

    <div class="go-card">
      <div class="go-group-title">内容</div>
      <div class="go-row"><span>播放历史</span><button class="act" @click="router.push('/history')">查看</button></div>
      <div class="go-row"><span>录制列表</span><button class="act" @click="router.push('/record-list')">查看</button></div>
      <div class="go-row"><span>检查更新</span><button class="act" :disabled="checking" @click="check">{{ checking ? '检查中…' : '检查更新' }}</button></div>
      <div class="go-row"><span>查看完整更新说明</span><button class="act" @click="router.push('/update-doc')">打开</button></div>
    </div>

    <div class="go-card">
      <div class="go-group-title">桌面专属</div>
      <div class="go-row"><span>文件关联</span><span class="sub">{{ assoc.length }} 种扩展名 · rank Alternate</span></div>
      <div class="go-row"><span>媒体键桥接</span><span class="on">SMTC / NowPlaying / MPRIS</span></div>
      <div class="go-row"><span>防休眠</span><span class="on">录制时开启</span></div>
      <div class="go-row"><span>开机自启</span><button class="act" @click="toast.show('由系统设置接管')">配置</button></div>
    </div>

    <div class="go-card">
      <div class="go-group-title">不适用项</div>
      <div class="go-row disabled"><span>沉浸式状态栏</span><span class="sub">无需设置</span></div>
      <div class="go-row disabled"><span>Doze / 唤醒锁</span><span class="sub">无需设置</span></div>
    </div>

    <div class="go-card meta">
      <div class="go-row"><span>当前版本</span><span class="tabular">{{ APP_VERSION }}</span></div>
      <div class="go-row"><span>构建号</span><span class="tabular">{{ BUILD_NUMBER }}</span></div>
    </div>
  </PageScaffold>
</template>

<style scoped>
select { background: var(--go-surface-2); color: var(--go-text); border: 1px solid var(--go-line); border-radius: 8px; padding: 6px 10px; font-size: 13px; }
.chips { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; max-width: 360px; }
.on { font-size: 12px; color: var(--go-accent); }
.sub { font-size: 12px; color: var(--go-text-3); }
.act { font-size: 13px; color: var(--go-accent); padding: 6px 10px; border-radius: 8px; }
.act:hover { background: var(--go-accent-8); }
.act:disabled { opacity: .4; cursor: default; }
.go-row.disabled { opacity: .45; }
.sample { font-size: 16px; color: var(--go-text-2); font-family: var(--go-font-lyric); }
.meta { margin-top: auto; }
</style>
