import { defineStore } from 'pinia'
import type { Lyric, Song } from '../api/types'

export const usePlayerStore = defineStore('player', {
  state: () => ({
    current: null as Song | null,
    queue: [] as Song[],
    playing: false,
    positionMs: 0,
    durationMs: 0,
    volume: 1,
    level: 'higher' as string,
    ctrlOpen: true,          // 只认右侧箭头，无自动收起
    lyric: { lines: [], hasTimeline: false } as Lyric,
    accent: '#6ea8fe',
  }),
  getters: {
    progress: (s) => (s.durationMs > 0 ? Math.min(1, s.positionMs / s.durationMs) : 0),
  },
  actions: {
    setCurrent(song: Song | null) { this.current = song; this.durationMs = song?.durationMs ?? 0; this.positionMs = 0 },
    setQueue(songs: Song[]) { this.queue = songs },
    setAccent(hex: string) { this.accent = hex },
  },
})
