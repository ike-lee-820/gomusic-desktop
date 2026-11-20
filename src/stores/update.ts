import { defineStore } from 'pinia'
import { ipc } from '../api/ipc'
import type { UpdateDoc } from '../api/types'

/** 应用启动即自动检查；网络/代理异常不阻断界面，静默切离线内置说明。 */
export const useUpdateStore = defineStore('update', {
  state: () => ({
    checking: false,
    latest: null as UpdateDoc | null,
    offline: null as UpdateDoc | null,
    error: '',
    dismissed: false,
  }),
  getters: {
    hasUpdate: (s) => !!s.latest?.hasUpdate && !s.dismissed,
    doc: (s) => s.latest ?? s.offline,
  },
  actions: {
    async check(force = false) {
      if (this.checking) return
      this.checking = true
      this.error = ''
      try {
        this.latest = await ipc.updateCheck(force)
      } catch {
        this.error = '更新检查暂不可用，已切换离线更新说明'
        await this.ensureOffline()
      } finally {
        this.checking = false
      }
    },
    async ensureOffline() {
      if (this.offline) return
      try { this.offline = await ipc.updateNotes() } catch { /* 离线文档也不可用时保持空态 */ }
    },
    dismiss() { this.dismissed = true },
  },
})
