import { defineStore } from 'pinia'
import type { RecConfig, RecProgress, RecTask } from '../api/types'

export const defaultRecConfig: RecConfig = {
  ratio: '16:9', resolution: 1080, fps: 30, level: 'higher', bitrate: 6000,
  lyricSize: 36, infoSize: 16, infoZone: 0.34, lyricOffsetMs: 0,
  customLrc: null, fontSource: 'SOURCE_HAN_SANS', wght: 400, showTrans: true,
  style: 'CLASSIC', background: 'GAUSS', vocal: 'ORIGINAL', reduceStrength: 0,
}

export const useRecorderStore = defineStore('recorder', {
  state: () => ({
    config: { ...defaultRecConfig } as RecConfig,
    availableLevels: [] as string[],
    tasks: [] as RecTask[],
    progress: null as RecProgress | null,
    running: false,
  }),
  actions: {
    setLevels(levels: string[]) {
      this.availableLevels = levels
      if (levels.length && !levels.includes(this.config.level)) this.config.level = levels[0]
    },
    onProgress(p: RecProgress) { this.progress = p },
    reset() { this.progress = null; this.running = false },
  },
})
