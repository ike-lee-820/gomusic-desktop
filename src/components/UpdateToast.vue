<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useUpdateStore } from '../stores/update'
import { ipc } from '../api/ipc'

const update = useUpdateStore()
const router = useRouter()

async function open(url?: string | null) {
  if (!url) return
  try { await ipc.openExternal(url) } catch { /* 加速失败时可用直连按钮 */ }
}
</script>

<template>
  <div v-if="update.hasUpdate" class="update-toast">
    <div class="head">
      <span class="dot"></span>
      <span class="msg">发现新版本 <b>{{ update.latest?.version }}</b></span>
      <button class="x" title="关闭" @click="update.dismiss()">×</button>
    </div>
    <div class="actions">
      <button class="primary" @click="router.push('/update-doc')">查看更新说明</button>
      <button v-if="update.latest?.downloadUrl" @click="open(update.latest?.downloadUrl)">下载更新（加速）</button>
      <button v-if="update.latest?.fallbackUrl" class="link" @click="open(update.latest?.fallbackUrl)">直连</button>
    </div>
  </div>
</template>

<style scoped>
.update-toast {
  position: fixed; left: 50%; bottom: 24px; transform: translateX(-50%);
  z-index: 90; min-width: 340px; max-width: 520px;
  background: var(--go-surface-2); border: 1px solid var(--go-line);
  border-radius: 10px; box-shadow: var(--shadow-float);
  padding: 12px 16px; font-size: 13px;
}
.head { display: flex; align-items: center; gap: 8px; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--go-accent); flex: none; }
.msg { flex: 1; color: var(--go-text); }
.msg b { font-variant-numeric: tabular-nums; color: var(--go-accent); }
.x { width: 24px; height: 24px; color: var(--go-text-3); }
.x:hover { color: var(--go-text); }
.actions { display: flex; align-items: center; gap: 8px; margin-top: 12px; }
.actions button {
  height: 30px; padding: 0 12px; border-radius: 8px;
  border: 1px solid var(--go-line); color: var(--go-text-2); font-size: 12px;
}
.actions button:hover { background: var(--go-surface); color: var(--go-text); }
.actions .primary { background: var(--go-accent-8); color: var(--go-accent); border-color: transparent; }
.actions .link { border-color: transparent; color: var(--go-text-3); }
</style>
