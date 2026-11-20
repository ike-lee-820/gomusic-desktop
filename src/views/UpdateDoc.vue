<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { useUpdateStore } from '../stores/update'
import { APP_VERSION } from '../version'

const router = useRouter()
const update = useUpdateStore()
const loading = ref(false)
const md = new MarkdownIt({ html: false, linkify: false, breaks: true })

/** 对外文案只有「新增 / 修复」两栏，其余段落一律不渲染。 */
function onlyNewFix(body: string): string {
  const keep: string[] = []
  let section = ''
  for (const line of body.split('\n')) {
    const h = /^#{1,6}\s*(.+?)\s*$/.exec(line)
    if (h) {
      const t = h[1].replace(/[:：].*$/, '').trim()
      section = /^(新增|修复)$/.test(t) ? t : ''
      if (section) keep.push(`## ${section}`)
      continue
    }
    if (section) keep.push(line)
  }
  return keep.join('\n').trim() || body
}

const html = computed(() => DOMPurify.sanitize(md.render(onlyNewFix(update.doc?.body ?? ''))))
const version = computed(() => update.doc?.version || APP_VERSION)
const published = computed(() => update.doc?.publishedAt?.slice(0, 10) || '')

async function reload() {
  loading.value = true
  await update.check(true)
  await update.ensureOffline()
  loading.value = false
}

onMounted(async () => {
  loading.value = true
  if (!update.latest) await update.check(true)
  await update.ensureOffline()
  loading.value = false
})
</script>

<template>
  <div class="page">
    <div class="head">
      <button class="back" @click="router.back()">←</button>
      <h1>更新说明</h1>
    </div>
    <div class="scroll no-scrollbar">
      <div class="go-card doc-card">
        <div class="meta">
          <span>版本 {{ version }}</span>
          <span v-if="published"> · {{ published }}</span>
          <span v-if="update.error" class="err"> · {{ update.error }}</span>
        </div>
        <div v-if="loading" class="loading">载入中…</div>
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div v-else class="md selectable" v-html="html"></div>
        <div class="foot">
          <button :disabled="loading" @click="reload">{{ loading ? '载入中…' : '重新载入' }}</button>
        </div>
      </div>
      <div class="tail"></div>
    </div>
  </div>
</template>

<style scoped>
.page { height: 100%; display: flex; flex-direction: column; }
.head { flex: none; height: 56px; display: flex; align-items: center; gap: 12px; padding: 0 24px; }
.back { width: 32px; height: 32px; border-radius: 8px; color: var(--go-text-2); }
.back:hover { background: var(--go-surface-2); }
h1 { margin: 0; font-size: 16px; font-weight: 600; }
.scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 24px 32px; }
.doc-card { max-width: 900px; }
.meta { font-size: 13px; color: var(--go-text-3); margin-bottom: 20px; }
.err { color: var(--go-warn); }
.loading { font-size: 12px; color: var(--go-text-3); }
.md { font-size: 14px; line-height: 1.8; color: var(--go-text-2); }
.md :deep(h2) { font-size: 15px; font-weight: 600; color: var(--go-text); margin: 20px 0 8px; }
.md :deep(ul) { padding-left: 20px; margin: 0; }
.md :deep(li) { margin: 4px 0; }
.md :deep(code) { background: var(--go-surface-2); border-radius: 4px; padding: 1px 5px; font-size: 12px; }
.foot { display: flex; justify-content: flex-end; margin-top: 24px; }
.foot button { font-size: 13px; color: var(--go-accent); }
.foot button:disabled { opacity: .4; cursor: default; }
.tail { height: 32px; }
</style>
