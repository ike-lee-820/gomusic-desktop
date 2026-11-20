<script setup lang="ts">
import { onMounted } from 'vue'
import PageScaffold from '../components/PageScaffold.vue'
import ProgressBar from '../components/ProgressBar.vue'
import { ipc } from '../api/ipc'
import { toast, confirm } from '../composables/overlay'
import { useRecorderStore } from '../stores/recorder'

const rec = useRecorderStore()

async function load() { try { rec.tasks = await ipc.recList() } catch { /* 离线 */ } }
async function remove(id: string) { await ipc.recDelete(id).catch(() => {}); await load() }
async function clear() {
  confirm.ask('清空历史', '将删除全部已完成的录制记录，此操作不可撤销。', async () => {
    for (const t of rec.tasks) await ipc.recDelete(t.id).catch(() => {})
    await load(); toast.show('已清空历史')
  })
}
onMounted(load)
</script>

<template>
  <PageScaffold title="录制列表">
    <template #ops><button @click="clear">清空历史</button></template>
    <p v-if="!rec.tasks.length" class="empty">暂无录制任务，去「录制视频」开始生成</p>
    <div v-for="t in rec.tasks" :key="t.id" class="go-card task">
      <div class="cover"></div>
      <div class="body">
        <div class="top"><span class="name">{{ t.config.level }} · {{ t.config.ratio }} · {{ t.config.style }}</span>
          <span class="chip" :class="t.state">{{ t.state }}</span></div>
        <ProgressBar :value="t.percent" />
        <div class="detail">{{ t.detail }} · {{ t.percent }}%<span v-if="t.outputPath" class="tabular"> · 已完成</span></div>
        <div class="acts">
          <button v-if="t.state === 'RUNNING'" @click="ipc.recCancel(t.id).then(load)">取消</button>
          <button v-if="t.outputPath" @click="toast.show('打开成品')">打开成品</button>
          <button @click="toast.show('打开所在目录')">打开所在目录</button>
          <button class="danger" @click="remove(t.id)">删除</button>
        </div>
      </div>
    </div>
  </PageScaffold>
</template>

<style scoped>
.empty { color: var(--go-text-3); font-size: 14px; text-align: center; margin-top: 40px; }
.task { display: flex; gap: 16px; align-items: flex-start; }
.cover { width: 48px; height: 48px; border-radius: 8px; background: var(--go-surface-2); flex: none; }
.body { flex: 1; min-width: 0; }
.top { display: flex; justify-content: space-between; margin-bottom: 10px; }
.name { font-size: 14px; }
.chip { font-size: 11px; padding: 3px 10px; border-radius: var(--r-chip); background: var(--go-accent-8); color: var(--go-accent); }
.chip.FAILED, .chip.CANCELLED { color: var(--go-danger); background: rgba(229,83,75,.12); }
.detail { font-size: 11px; color: var(--go-text-3); margin-top: 8px; }
.acts { display: flex; gap: 12px; margin-top: 12px; }
.acts button { font-size: 12px; color: var(--go-accent); }
.acts .danger { color: var(--go-danger); }
</style>
