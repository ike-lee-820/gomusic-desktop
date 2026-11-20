<script setup lang="ts">
import { onMounted, ref } from 'vue'
import PageScaffold from '../components/PageScaffold.vue'
import Chip from '../components/Chip.vue'
import ContextMenu from '../components/ContextMenu.vue'
import type { MenuItem, ContextMenuHandle } from '../components/menu'
import { ipc } from '../api/ipc'
import type { Song } from '../api/types'
import { toast } from '../composables/overlay'
import { usePlayerStore } from '../stores/player'

const player = usePlayerStore()
const kw = ref('')
const tab = ref(1)
const tabs = [{ t: 1, n: '单曲' }, { t: 10, n: '专辑' }, { t: 100, n: '歌手' }, { t: 1000, n: '歌单' }]
const hot = ['十年', '周杰伦', '陈奕迅', '新碟', '华语']
const rows = ref<Song[]>([])
const selected = ref<number | null>(null)
const loading = ref(false)
const offset = ref(0)
const menu = ref<ContextMenuHandle | null>(null)

async function load(reset = false) {
  if (!kw.value.trim() || loading.value) return
  if (reset) { offset.value = 0; rows.value = [] }
  loading.value = true
  try {
    const r = await ipc.search(kw.value.trim(), tab.value, 20, offset.value)
    rows.value = reset ? r.songs : [...rows.value, ...r.songs]
    if (r.hasMore) offset.value += 20
  } catch { toast.error('搜索暂不可用') }
  loading.value = false
}
function onScroll(e: Event) {
  const el = e.target as HTMLElement
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 80) void load()
}
function play(s: Song) { player.setCurrent(s); toast.show(`播放 ${s.name}`) }
function ctx(e: MouseEvent, s: Song) {
  const items: MenuItem[] = [
    { label: '播放', action: () => play(s) },
    { label: '下一首播放', action: () => toast.show('已加入队列') },
    { label: '复制名称', action: () => navigator.clipboard?.writeText(s.name) },
    { label: '查看专辑', action: () => toast.show('专辑页') },
    { label: '查看歌手', action: () => toast.show('歌手页') },
    { label: '下载', action: () => toast.show('已加入下载') },
  ]
  menu.value?.show(e.clientX, e.clientY, items)
}
onMounted(() => { kw.value = '十年'; void load(true) })
</script>

<template>
  <PageScaffold title="搜索">
    <div class="search-bar">
      <span class="ic">⌕</span>
      <input v-model="kw" placeholder="搜索歌曲 / 专辑 / 歌手" @keyup.enter="load(true)" />
      <div class="hots"><Chip v-for="h in hot" :key="h" :label="h" @select="kw = h; load(true)" /></div>
    </div>
    <div class="tabs">
      <button v-for="t in tabs" :key="t.t" :class="{ on: tab === t.t }" @click="tab = t.t; load(true)">{{ t.n }}</button>
    </div>
    <div class="list no-scrollbar" @scroll="onScroll">
      <div v-for="(s, i) in rows" :key="s.id + '-' + i" class="row"
           :class="{ sel: selected === i }"
           @click="selected = i" @dblclick="play(s)" @contextmenu.prevent="ctx($event, s)">
        <span class="idx tabular">{{ String(i + 1).padStart(2, '0') }}</span>
        <div class="cover"></div>
        <div class="txt"><div class="name">{{ s.name }}</div><div class="sub">{{ s.artist }} · {{ s.album }}</div></div>
        <span class="badge">{{ s.level || 'standard' }}</span>
        <button class="more" @click.stop="ctx($event, s)">⋯</button>
      </div>
      <div v-if="loading" class="loading">载入中…</div>
      <div v-if="!loading && !rows.length" class="empty">输入关键词开始搜索</div>
    </div>
    <ContextMenu ref="menu" />
  </PageScaffold>
</template>

<style scoped>
.search-bar { display: flex; align-items: center; gap: 12px; height: 44px; padding: 0 12px; background: var(--go-surface); border: 1px solid var(--go-line); border-radius: 10px; flex: none; }
.ic { color: var(--go-text-3); font-size: 18px; }
input { flex: 1; background: none; border: 0; outline: none; font-size: 14px; }
.hots { display: flex; gap: 8px; }
.tabs { display: flex; gap: 24px; height: 40px; align-items: center; flex: none; }
.tabs button { font-size: 14px; color: var(--go-text-2); height: 100%; position: relative; }
.tabs button.on { color: var(--go-text); }
.tabs button.on::after { content: ''; position: absolute; bottom: 0; left: 0; right: 0; height: 2px; background: var(--go-accent); }
.list { flex: 1; min-height: 0; overflow-y: auto; }
.row { display: flex; align-items: center; gap: 12px; height: 64px; border-bottom: 1px solid var(--go-line); padding: 0 8px; border-radius: 8px; }
.row:hover, .row.sel { background: var(--go-surface-2); }
.idx { width: 24px; font-size: 12px; color: var(--go-text-3); }
.cover { width: 44px; height: 44px; border-radius: 8px; background: var(--go-surface-2); flex: none; }
.txt { flex: 1; min-width: 0; }
.name { font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sub { font-size: 12px; color: var(--go-text-2); margin-top: 3px; }
.badge { font-size: 11px; color: var(--go-text-3); border: 1px solid var(--go-line); border-radius: var(--r-chip); padding: 2px 8px; }
.more { width: 32px; color: var(--go-text-3); }
.loading, .empty { text-align: center; color: var(--go-text-3); font-size: 12px; padding: 20px; }
</style>
