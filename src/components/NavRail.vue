<script setup lang="ts">
import { APP_VERSION, BUILD_NUMBER } from '../version'

const items = [
  { to: '/', label: '播放', d: 'M8 5v14l11-7z' },
  { to: '/search', label: '搜索', d: 'M11 4a7 7 0 1 0 4.2 12.6L20 21l1.4-1.4-4.8-4.8A7 7 0 0 0 11 4zm0 2a5 5 0 1 1 0 10 5 5 0 0 1 0-10z' },
  { to: '/discover', label: '发现', d: 'M12 3l2.6 5.9 6.4.5-4.9 4.2 1.5 6.2L12 16.5 6.4 19.8l1.5-6.2L3 9.4l6.4-.5z' },
  { to: '/library', label: '曲库', d: 'M4 5h16v3H4zm0 5h16v9H4zm3 3v5h10v-5z' },
  { to: '/history', label: '历史', d: 'M12 4a8 8 0 1 0 7.6 10.5l-1.9-.6A6 6 0 1 1 12 6v3l4-4-4-4z' },
  { to: '/recorder', label: '录制', d: 'M12 5a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm0 3a4 4 0 1 1 0 8 4 4 0 0 1 0-8z' },
  { to: '/settings', label: '设置', d: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm9 4l-2-1.2.3-2.3-2.2-.9-1.3 1.9-2.1-.6L13 6h-2l-.7 2.9-2.1.6-1.3-1.9-2.2.9.3 2.3L3 12l2 1.2-.3 2.3 2.2.9 1.3-1.9 2.1.6L11 18h2l.7-2.9 2.1-.6 1.3 1.9 2.2-.9-.3-2.3z' },
]
</script>

<template>
  <nav class="rail">
    <div class="items">
      <router-link v-for="it in items" :key="it.to" :to="it.to" class="item" active-class="is-active" exact-active-class="is-active">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
          <path :d="it.d" />
        </svg>
        <span class="label">{{ it.label }}</span>
      </router-link>
    </div>
    <span class="ver tabular" :title="`构建号 ${BUILD_NUMBER}`">{{ APP_VERSION }}</span>
  </nav>
</template>

<style scoped>
.rail {
  width: var(--navrail-w); flex: none; height: 100%;
  background: var(--go-surface); border-right: 1px solid var(--go-line);
  display: flex; flex-direction: column; align-items: center;
  justify-content: space-between; padding: var(--sp-3) 0;
}
.items { display: flex; flex-direction: column; gap: var(--sp-2); width: 100%; align-items: center; }
.item {
  position: relative; width: 64px; height: 56px; border-radius: 10px;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 4px; color: var(--go-text-2);
}
.item:hover { background: var(--go-surface-2); }
.item.is-active { background: var(--go-surface-2); color: var(--go-accent); }
.item.is-active::before {
  content: ''; position: absolute; left: -6px; top: 12px; bottom: 12px;
  width: 3px; border-radius: 2px; background: var(--go-accent);
}
.label { font-size: 10px; }
.ver { font-size: 10px; color: var(--go-text-3); }
</style>
