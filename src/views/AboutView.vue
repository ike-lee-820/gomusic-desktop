<script setup lang="ts">
import { useRouter } from 'vue-router'
import PageScaffold from '../components/PageScaffold.vue'
import { ipc } from '../api/ipc'
import { APP_VERSION, BUILD_NUMBER, BUILD_TAG } from '../version'

const router = useRouter()
const repo = 'https://github.com/ike-lee-820/gomusic-desktop'
</script>

<template>
  <PageScaffold title="关于">
    <div class="wrap">
      <div class="logo">G</div>
      <h2>GoMusic</h2>
      <div class="ver tabular">版本 {{ APP_VERSION }} · 构建号 {{ BUILD_NUMBER }} · {{ BUILD_TAG }}</div>
      <div class="go-card info">
        <div class="go-row"><span>更新说明</span><button class="act" @click="router.push('/update-doc')">查看</button></div>
        <div class="go-row"><span>GitHub</span><button class="act" @click="ipc.openExternal(repo).catch(() => {})">打开仓库</button></div>
        <div class="go-row"><span>仅横屏</span><span class="sub">1280×800 / 最小 960×600 · 左导航 76px</span></div>
        <div class="go-row"><span>凭据轮换提示</span><span class="sub">凭据只存 CI Secrets / keyring</span></div>
      </div>
    </div>
  </PageScaffold>
</template>

<style scoped>
.wrap { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; }
.logo { width: 64px; height: 64px; border-radius: 16px; display: grid; place-items: center; font-size: 32px; font-weight: 600; background: var(--go-accent-8); color: var(--go-accent); }
h2 { margin: 4px 0 0; font-size: 22px; font-weight: 600; }
.ver { font-size: 12px; color: var(--go-text-3); }
.info { width: 420px; margin-top: 20px; }
.act { font-size: 13px; color: var(--go-accent); }
.sub { font-size: 12px; color: var(--go-text-3); }
</style>
