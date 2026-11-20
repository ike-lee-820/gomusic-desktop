<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import ContextMenu from '../components/ContextMenu.vue'
import type { MenuItem, ContextMenuHandle } from '../components/menu'
import ProgressBar from '../components/ProgressBar.vue'
import { ipc } from '../api/ipc'
import { toast } from '../composables/overlay'
import { usePlayerStore } from '../stores/player'
import type { ArtistDesc, Song } from '../api/types'

const route = useRoute()
const router = useRouter()
const player = usePlayerStore()
const kind = computed(() => String(route.meta.kind ?? 'history'))
const id = computed(() => Number(route.params.id ?? 0))
const title = computed(() => String(route.meta.title ?? ''))
const desc = ref<ArtistDesc | null>(null)
const logged = ref(false)
const rows = ref<Song[]>([])
const menu = ref<ContextMenuHandle | null>(null)

const topics = [
  { title: '金曲奖前瞻', summary: '那些年陪我们长大的旋律', tags: ['金曲奖', '周杰伦'] },
  { title: '幕后故事', summary: '录音室里的一个小片段', tags: ['幕后'] },
]

async function loadDesc() {
  try {
    desc.value = kind.value === 'album-desc' ? await ipc.albumDesc(id.value)
      : kind.value === 'artist-desc' ? await ipc.artistDesc(id.value)
      : await ipc.artistDetail(id.value)
  } catch { desc.value = null }
}
async function loadRows() {
  try {
    rows.value = kind.value === 'history' ? await ipc.historyList(100)
      : kind.value === 'local' ? await ipc.localScan([])
      : player.queue
  } catch { rows.value = [] }
}
function ctx(e: MouseEvent, s: Song) {
  const items: MenuItem[] = [
    { label: '播放', action: () => { player.setCurrent(s); toast.show(`播放 ${s.name}`) } },
    { label: kind.value === 'history' ? '从历史移除' : '打开所在目录', action: () => toast.show('已处理') },
    { label: '复制名称', action: () => navigator.clipboard?.writeText(s.name) },
  ]
  menu.value?.show(e.clientX, e.clientY, items)
}
function onDrop(e: DragEvent) {
  e.preventDefault()
  const paths = [...(e.dataTransfer?.files ?? [])].map((f) => f.name)
  toast.show(paths.length ? `已送入 ${paths.length} 个文件` : '未识别文件')
}
onMounted(() => { if (kind.value.includes('desc')) void loadDesc(); void loadRows() })
</script>

<template>
  <PageScaffold :title="title">
    <!-- 专辑 / 歌手简介页：左大图 + 右唯一滚动容器 -->
    <div v-if="kind === 'album-desc' || kind === 'artist-desc'" class="desc">
      <div class="left">
        <div class="big"></div>
        <button class="save" @click="toast.show('已保存到 ~/Downloads/GoMusic/图片/')">保存图片</button>
      </div>
      <div class="right no-scrollbar">
        <h2>{{ kind === 'album-desc' ? '专辑标题' : '歌手名字' }}</h2>
        <div class="go-card">
          <div class="go-group-title">简介</div>
          <div v-if="logged && desc" class="text selectable">{{ desc.briefDesc }}</div>
          <div v-else class="placeholder">登录后查看{{ kind === 'album-desc' ? '专辑' : '歌手' }}简介 <button @click="router.push('/login')">去登录</button></div>
        </div>
        <template v-if="kind === 'artist-desc'">
          <div v-for="(s, i) in (desc?.sections ?? [{ ti: '人物简介', txt: '详细信息由 /artist/desc 免登录返回，超长按段懒加载。' }])" :key="i" class="go-card section">
            <div class="st">{{ s.ti }}</div><div class="text selectable">{{ s.txt }}</div>
          </div>
          <div class="go-card">
            <div class="go-group-title">相关专题（不渲染 HTML 正文与配图）</div>
            <div v-for="t in topics" :key="t.title" class="topic">
              <span>{{ t.title }} · {{ t.summary }}</span>
              <div class="chips"><Chip v-for="tag in t.tags" :key="tag" :label="tag" /></div>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- 专辑 / 歌手主页 -->
    <template v-else-if="kind === 'album'">
      <div class="album-head"><div class="cover200"></div><div><h2>专辑名</h2><p>歌手 · 曲目数 / 年份</p></div></div>
      <div class="go-card"><div class="go-group-title">曲目</div>
        <div v-for="i in 10" :key="i" class="go-row"><span class="tabular">{{ i }}</span><span>歌曲 {{ i }}</span><span class="sub tabular">04:12</span><span class="sub">standard</span><button @click="router.push('/album-desc/' + id)">⋯</button></div>
      </div>
    </template>
    <template v-else-if="kind === 'artist'">
      <div class="artist-hero" @click="router.push('/artist-desc/' + id)"><h2>歌手名字</h2></div>
      <div class="go-card"><div class="go-group-title">热门歌曲</div>
        <div v-for="i in 8" :key="i" class="go-row"><span class="tabular">{{ i }}</span><span>热门歌曲 {{ i }}</span><span class="sub tabular">04:12</span></div>
      </div>
      <div class="go-card"><div class="go-group-title">专辑</div><div class="albums"><div v-for="i in 4" :key="i" class="mini"></div></div></div>
    </template>

    <!-- 播放历史 / 本地音乐 / 队列 -->
    <template v-else>
      <div v-if="kind === 'local'" class="drop" @dragover.prevent @drop="onDrop">拖放音频文件到窗口，或用「打开方式」送入 · 点封面/歌手不跳转</div>
      <div class="go-card">
        <div class="go-group-title">{{ kind === 'history' ? '在线 + 本地统一' : kind === 'local' ? '本地索引' : '右侧抽屉 360px · 拖拽排序' }}</div>
        <div v-for="(s, i) in rows" :key="s.id + '-' + i" class="go-row song" @contextmenu.prevent="ctx($event, s)">
          <span class="tabular idx">{{ String(i + 1).padStart(2, '0') }}</span>
          <div class="mini"></div>
          <div class="txt"><div>{{ s.name || ('本地音乐 ' + (i + 1)) }}</div><div class="sub">{{ s.artist || '未知歌手' }}</div></div>
          <span class="chip">{{ s.source === 'LOCAL' ? '本地' : '在线' }}</span>
          <button @click="ctx($event, s)">⋯</button>
        </div>
        <div v-if="!rows.length" class="empty">暂无记录</div>
      </div>
      <ProgressBar :value="player.positionMs" :total="player.durationMs" interactive @seek="r => player.positionMs = r * player.durationMs" />
    </template>
    <ContextMenu ref="menu" />
  </PageScaffold>
</template>

<style scoped>
.desc { display: flex; gap: 24px; height: 100%; min-height: 0; }
.left { flex: 0 0 40%; display: flex; flex-direction: column; gap: 12px; }
.big { width: 100%; max-width: 360px; aspect-ratio: 1; border-radius: 16px; background: var(--go-surface-2); }
.save { align-self: flex-start; font-size: 13px; color: var(--go-accent); }
.right { flex: 1; min-width: 0; overflow-y: auto; padding-bottom: 32px; display: flex; flex-direction: column; gap: 16px; }
h2 { margin: 0; font-size: 26px; font-weight: 600; }
.text { font-size: 14px; line-height: 1.8; color: var(--go-text-2); white-space: pre-wrap; }
.placeholder { font-size: 13px; color: var(--go-text-3); }
.placeholder button { color: var(--go-accent); margin-left: 8px; }
.st { font-size: 13px; color: var(--go-accent); margin-bottom: 8px; }
.topic { display: flex; justify-content: space-between; gap: 12px; padding: 8px 0; font-size: 13px; color: var(--go-text-2); }
.chips { display: flex; gap: 6px; }
.album-head { display: flex; gap: 24px; align-items: center; }
.cover200 { width: 200px; height: 200px; border-radius: 16px; background: var(--go-surface-2); flex: none; }
.album-head h2 { font-size: 22px; }
.album-head p { font-size: 14px; color: var(--go-text-2); }
.artist-hero { height: 220px; border-radius: 16px; background: linear-gradient(180deg, rgba(0,0,0,.45), transparent), var(--go-surface-2); display: flex; align-items: flex-end; padding: 20px; cursor: pointer; }
.artist-hero h2 { font-size: 24px; }
.albums { display: flex; gap: 12px; }
.mini { width: 44px; height: 44px; border-radius: 8px; background: var(--go-surface-2); flex: none; }
.drop { border: 1px dashed var(--go-line); border-radius: 16px; padding: 32px; text-align: center; font-size: 13px; color: var(--go-text-3); }
.song { cursor: default; }
.idx { width: 24px; font-size: 11px; color: var(--go-text-3); }
.txt { flex: 1; min-width: 0; }
.sub { font-size: 12px; color: var(--go-text-3); }
.chip { font-size: 11px; color: var(--go-accent); background: var(--go-accent-8); border-radius: var(--r-chip); padding: 3px 10px; }
.empty { text-align: center; font-size: 12px; color: var(--go-text-3); padding: 16px; }
</style>
