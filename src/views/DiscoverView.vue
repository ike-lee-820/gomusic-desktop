<script setup lang="ts">
import { useRouter } from 'vue-router'
import PageScaffold from '../components/PageScaffold.vue'
import { toast } from '../composables/overlay'

const router = useRouter()
const blocks = [
  { title: '新碟上架', desc: '本周新发行的专辑', items: ['范特西', '七里香', '叶惠美', '依然'] },
  { title: '新歌速递', desc: '最新单曲', items: ['晴天', '十年', '起风了', '孤勇者'] },
  { title: '榜单', desc: '热歌 / 新歌 / 原创', items: ['热歌榜', '新歌榜', '飙升榜', '原创榜'] },
  { title: '热词', desc: '大家都在搜', items: ['周杰伦', '陈奕迅', '邓紫棋', '毛不易'] },
]
</script>

<template>
  <PageScaffold title="发现">
    <div class="grid">
      <div v-for="b in blocks" :key="b.title" class="go-card block">
        <div class="head"><h2>{{ b.title }}</h2><span>{{ b.desc }}</span></div>
        <div class="cards">
          <button v-for="it in b.items" :key="it" class="item" @click="router.push('/search')">
            <div class="cover"></div><span>{{ it }}</span>
          </button>
        </div>
      </div>
    </div>
    <div class="go-card">
      <div class="go-group-title">数据</div>
      <div class="go-row"><span>来源</span><span class="sub">网易云主源 · Rust net/ 统一请求</span></div>
      <div class="go-row"><span>缓存</span><button class="act" @click="toast.show('已刷新')">刷新</button></div>
    </div>
  </PageScaffold>
</template>

<style scoped>
.grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; }
.block h2 { margin: 0; font-size: 14px; font-weight: 600; }
.head { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 16px; }
.head span { font-size: 11px; color: var(--go-text-3); }
.cards { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
.item { display: flex; flex-direction: column; align-items: center; gap: 8px; font-size: 12px; color: var(--go-text-2); }
.item:hover { color: var(--go-text); }
.cover { width: 100%; aspect-ratio: 1; max-width: 120px; border-radius: 12px; background: var(--go-surface-2); border: 1px solid var(--go-line); }
.sub { font-size: 12px; color: var(--go-text-3); }
.act { font-size: 13px; color: var(--go-accent); }
</style>
