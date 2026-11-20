import { createRouter, createWebHistory } from 'vue-router'
import PlayerWide from '../views/PlayerWide.vue'
import SearchView from '../views/SearchView.vue'
import DiscoverView from '../views/DiscoverView.vue'
import EqualizerView from '../views/EqualizerView.vue'
import SurroundView from '../views/SurroundView.vue'
import SettingsView from '../views/SettingsView.vue'
import RecorderView from '../views/RecorderView.vue'
import RecordListView from '../views/RecordListView.vue'
import UpdateDoc from '../views/UpdateDoc.vue'
import LoginView from '../views/LoginView.vue'
import AboutView from '../views/AboutView.vue'
import DetailPage from '../views/DetailPage.vue'

const detail = (path: string, title: string, kind: string) => ({
  path, component: DetailPage, meta: { title, kind, standalone: true },
})

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: PlayerWide, meta: { title: '播放', standalone: false } },
    { path: '/search', component: SearchView, meta: { title: '搜索', standalone: true } },
    { path: '/discover', component: DiscoverView, meta: { title: '发现', standalone: true } },
    detail('/library', '曲库', 'local'),
    detail('/history', '播放历史', 'history'),
    detail('/queue', '播放队列', 'queue'),
    detail('/album/:id', '专辑', 'album'),
    detail('/album-desc/:id', '专辑简介', 'album-desc'),
    detail('/artist/:id', '歌手', 'artist'),
    detail('/artist-desc/:id', '歌手简介', 'artist-desc'),
    { path: '/equalizer', component: EqualizerView, meta: { title: '均衡器', standalone: true } },
    { path: '/surround', component: SurroundView, meta: { title: '3D 环绕', standalone: true } },
    { path: '/settings', component: SettingsView, meta: { title: '设置', standalone: true } },
    { path: '/recorder', component: RecorderView, meta: { title: '录制视频', standalone: true } },
    { path: '/record-list', component: RecordListView, meta: { title: '录制列表', standalone: true } },
    { path: '/update-doc', component: UpdateDoc, meta: { title: '更新说明', standalone: true } },
    { path: '/login', component: LoginView, meta: { title: '登录 / 验证', standalone: true } },
    { path: '/about', component: AboutView, meta: { title: '关于', standalone: true } },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
})

export default router
