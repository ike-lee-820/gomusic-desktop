import { createRouter, createWebHistory } from 'vue-router'
import PlayerWide from '../views/PlayerWide.vue'
import Placeholder from '../views/Placeholder.vue'

const page = (path: string, title: string, note: string) => ({
  path,
  component: Placeholder,
  meta: { title, note, standalone: true },
})

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: PlayerWide, meta: { title: '播放', standalone: false } },
    page('/search', '搜索', '顶部搜索框 44 / 标签行 40 / 虚拟列表行高 64，双击播放、单击选中'),
    page('/discover', '发现', '新碟 / 新歌 / 榜单 / 热词 四块卡片'),
    page('/library', '曲库', '本地音乐：拖放或「打开方式」送入，点封面/歌手不跳转'),
    page('/history', '播放历史', '在线 + 本地统一，行高 64 虚拟列表'),
    page('/queue', '播放队列', '右侧抽屉 360px，行高 56，拖拽排序'),
    page('/album/:id', '专辑', '左封面 200×200，右信息 + 曲目列表行高 56'),
    page('/album-desc/:id', '专辑简介', '简介仅登录显示；大图右键保存 / 打开所在目录'),
    page('/artist/:id', '歌手', '顶部大图高 220 + 热门歌曲 + 专辑横卡'),
    page('/artist-desc/:id', '歌手简介', 'briefDesc + introduction 分段懒加载，topicData 只列标题'),
    page('/equalizer', '均衡器', '9 预设 + 强度 + 10 段推子 + 设备校准，30ms 淡变'),
    page('/surround', '3D 环绕', '独立 DSP，150Hz 以下不空间化，输出软限幅 −1dBFS'),
    page('/settings', '设置', '播放 / 网络 / 外观 / 内容 / 桌面专属 / 不适用项'),
    page('/recorder', '录制视频', '预览即成品：同一 RecRenderer canvas；三段式进度'),
    page('/record-list', '录制列表', '进行中置顶 + 历史倒序，进度三处同步'),
    page('/update-doc', '更新说明', 'markdown-it + DOMPurify，离线完整，仅「修复 / 新增」'),
    page('/login', '登录 / 验证', '卡片宽 420 居中；风控用系统浏览器打开验证页'),
    page('/about', '关于', '版本 26.11.36 / 构建号 / 更新说明入口 / GitHub 链接'),
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
})

export default router
