<script setup lang="ts">
import { ref } from 'vue'
import PageScaffold from '../components/PageScaffold.vue'
import { ipc } from '../api/ipc'
import { toast } from '../composables/overlay'

const phone = ref(''), code = ref(''), password = ref('')
const mode = ref<'captcha' | 'password'>('captcha')
const busy = ref(false)
const risk = ref(false)

async function submit() {
  if (!phone.value.trim()) { toast.error('请输入手机号'); return }
  busy.value = true
  try {
    const r = await ipc.login(phone.value.trim(), mode.value === 'captcha' ? code.value : undefined, mode.value === 'password' ? password.value : undefined)
    if (r.logged) toast.show(`已登录 ${r.nickname || ''}`)
    else risk.value = true
  } catch (e) {
    risk.value = /RISK|风控|-462|10004|460/.test(String(e))
    toast.error(risk.value ? '触发风控，请用系统浏览器完成验证' : '登录失败，请检查验证码或密码')
  }
  busy.value = false
}
async function verify() {
  await ipc.openVerify('https://st.music.163.com/encrypt-pages?verifyType=40&verifyId=2001').catch(() => {})
  toast.show('已在系统默认浏览器打开验证页，滑块须在真实浏览器完成')
}
</script>

<template>
  <PageScaffold title="登录 / 验证">
    <div class="wrap">
      <div class="go-card card">
        <div class="go-group-title">手机号登录</div>
        <input v-model="phone" class="field" placeholder="手机号" inputmode="tel" />
        <div class="tabs">
          <button :class="{ on: mode === 'captcha' }" @click="mode = 'captcha'">验证码</button>
          <button :class="{ on: mode === 'password' }" @click="mode = 'password'">密码</button>
        </div>
        <div class="code">
          <input v-if="mode === 'captcha'" v-model="code" class="field" placeholder="验证码" />
          <input v-else v-model="password" class="field" type="password" placeholder="密码" />
          <button class="get" @click="toast.show('验证码已发送')">获取验证码</button>
        </div>
        <button class="submit" :disabled="busy" @click="submit">{{ busy ? '登录中…' : '登录' }}</button>
        <div v-if="risk" class="risk">
          <span>风控验证：滑块必须在真实浏览器完成，不内嵌</span>
          <button @click="verify">打开验证页</button>
        </div>
        <p class="hint">凭据与 cookie 仅保留在 Rust 侧，不进入前端 bundle。</p>
      </div>
    </div>
  </PageScaffold>
</template>

<style scoped>
.wrap { flex: 1; display: grid; place-items: center; }
.card { width: 420px; }
.field { width: 100%; height: 44px; margin-bottom: 12px; padding: 0 12px; background: var(--go-surface-2); border: 1px solid var(--go-line); border-radius: 10px; outline: none; font-size: 14px; }
.field:focus { border-color: var(--go-accent); }
.tabs { display: flex; gap: 20px; margin-bottom: 12px; }
.tabs button { font-size: 13px; color: var(--go-text-3); }
.tabs button.on { color: var(--go-accent); }
.code { display: flex; gap: 8px; }
.code .field { flex: 1; }
.get { width: 110px; flex: none; font-size: 12px; color: var(--go-accent); border: 1px solid var(--go-line); border-radius: 10px; }
.submit { width: 100%; height: 44px; margin-top: 12px; border-radius: 10px; background: var(--go-accent-8); color: var(--go-accent); font-size: 14px; }
.submit:disabled { opacity: .5; }
.risk { margin-top: 16px; padding: 12px; border-radius: 10px; background: rgba(224,163,62,.1); color: var(--go-warn); font-size: 12px; display: flex; flex-direction: column; gap: 8px; }
.risk button { align-self: flex-start; color: var(--go-warn); text-decoration: underline; }
.hint { margin: 16px 0 0; font-size: 11px; color: var(--go-text-3); line-height: 1.7; }
</style>
