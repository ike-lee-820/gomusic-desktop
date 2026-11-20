<script setup lang="ts">
import { ref } from 'vue'
import { toast, confirm } from '../composables/overlay'

const msg = ref(''), danger = ref(false), timer = ref<number | null>(null)
const dialog = ref<{ title: string; text: string; onOk: () => void } | null>(null)

toast.bind((text, isDanger) => {
  msg.value = text; danger.value = isDanger
  if (timer.value) window.clearTimeout(timer.value)
  timer.value = window.setTimeout(() => { msg.value = '' }, 2500)
})
confirm.bind((title, text, onOk) => { dialog.value = { title, text, onOk } })
function ok() { const d = dialog.value; dialog.value = null; d?.onOk() }
</script>

<template>
  <div v-if="msg" class="toast" :class="{ danger }">{{ msg }}</div>
  <div v-if="dialog" class="mask" @click.self="dialog = null">
    <div class="dialog">
      <h2>{{ dialog.title }}</h2>
      <p>{{ dialog.text }}</p>
      <div class="acts">
        <button @click="dialog = null">取消</button>
        <button class="primary" @click="ok">确认</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toast {
  position: fixed; left: 50%; bottom: 24px; transform: translateX(-50%); z-index: 130;
  background: var(--go-surface-2); border: 1px solid var(--go-line); border-left: 3px solid var(--go-accent);
  border-radius: 10px; padding: 10px 16px; font-size: 13px; box-shadow: var(--shadow-float);
}
.toast.danger { border-left-color: var(--go-danger); }
.mask { position: fixed; inset: 0; z-index: 140; background: rgba(0,0,0,.45); display: grid; place-items: center; }
.dialog { width: 420px; border-radius: 20px; background: var(--go-surface-2); border: 1px solid var(--go-line); padding: 24px; box-shadow: var(--shadow-float); }
h2 { margin: 0 0 12px; font-size: 16px; font-weight: 600; }
p { margin: 0 0 24px; font-size: 14px; line-height: 1.7; color: var(--go-text-2); }
.acts { display: flex; justify-content: flex-end; gap: 8px; }
.acts button { padding: 8px 16px; border-radius: 10px; font-size: 13px; color: var(--go-text-2); }
.acts button:hover { background: var(--go-surface); }
.acts .primary { color: var(--go-accent); background: var(--go-accent-8); }
</style>
