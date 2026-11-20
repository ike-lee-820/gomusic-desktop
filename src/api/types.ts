/** §5 数据模型的前端镜像，与 Rust serde 结构保持同步。 */
export type Source = 'NM' | 'LOCAL'

export interface Song {
  id: number
  name: string
  artist: string
  artistId?: number | null
  album: string
  albumId?: number | null
  picUrl?: string | null
  durationMs: number
  source: Source
  localPath?: string | null
  level?: string | null
}

export interface LyricLine { timeMs: number; text: string; trans?: string; roma?: string }
export interface Lyric { lines: LyricLine[]; hasTimeline: boolean }

export interface SongUrl { url: string; level: string; br: number; size: number; md5: string }
export interface SearchResult { songs: Song[]; total: number; hasMore: boolean }
export interface ArtistDesc { briefDesc: string; sections: { ti: string; txt: string }[]; topics: { title: string; summary: string; tags: string[] }[] }

export interface EqConfig { preset: string; strength: number; bands: { freq: number; gain: number; q: number }[]; hpf: number; lpf: number; deviceCalib: string; abLoudness: boolean }
export interface SurroundConfig { enabled: boolean; strength: number; scene: string; width: number; distance: number; taps: number[]; reverb: number; eco: boolean }

export type RecBg = 'GAUSS' | 'RAIN' | 'SNOW'
export interface RecConfig {
  ratio: string; resolution: number; fps: number; level: string; bitrate: number
  lyricSize: number; infoSize: number; infoZone: number; lyricOffsetMs: number
  customLrc?: string | null; fontSource: string; wght: number; showTrans: boolean
  style: 'CLASSIC' | 'DISC'; background: RecBg
  vocal: 'ORIGINAL' | 'REDUCE'; reduceStrength: number
}
export interface RecTask { id: string; state: string; phase: number; detail: string; percent: number; outputPath?: string | null; coverPath?: string | null; config: RecConfig }
export interface RecProgress { phase: number; detail: string; percent: number; frame: number; total: number }
export interface UpdateDoc {
  version: string
  body: string
  hasUpdate: boolean
  downloadUrl?: string | null
  fallbackUrl?: string | null
  publishedAt?: string | null
}
export interface Settings { [key: string]: unknown }
export interface LoginState { logged: boolean; userId?: number; nickname?: string; avatarUrl?: string }
