import { invoke, Channel } from '@tauri-apps/api/core'
import type {
  ArtistDesc, EqConfig, LoginState, Lyric, RecConfig, RecProgress, RecTask,
  SearchResult, Settings, Song, SongUrl, SurroundConfig, UpdateDoc,
} from './types'

/** 全部业务请求经 Rust 发出，前端不直接跨域，凭据不进入 JS 层。 */
export const ipc = {
  search: (keywords: string, type: number, limit = 20, offset = 0) =>
    invoke<SearchResult>('search', { keywords, type, type_: type, limit, offset }),

  getSongUrl: (id: number, level: string, exact = false) =>
    invoke<SongUrl>('get_song_url', { id, level, exact }),
  probeLevels: (id: number) => invoke<string[]>('probe_levels', { id }),
  getLyric: (id: number, force = false) => invoke<Lyric>('get_lyric', { id, force }),

  albumDesc: (id: number) => invoke<ArtistDesc>('album_desc', { id }),
  artistDesc: (id: number) => invoke<ArtistDesc>('artist_desc', { id }),
  artistDetail: (id: number) => invoke<ArtistDesc>('artist_detail', { id }),
  songDetail: (ids: number[]) => invoke<unknown>('song_detail', { ids }),
  translate: (text: string, from = 'auto', to = 'zh-CN') =>
    invoke<string>('translate', { text, from, to }),

  login: (phone: string, captcha?: string, password?: string) =>
    invoke<LoginState>('login', { phone, captcha, password }),
  openVerify: (verifyUrl: string) => invoke<void>('open_verify', { verifyUrl }),

  playerPlay: (uri?: string) => invoke<void>('player_play', { uri }),
  playerPause: () => invoke<void>('player_pause'),
  playerSeek: (positionMs: number) => invoke<void>('player_seek', { positionMs }),
  playerNext: () => invoke<void>('player_next'),
  playerPrev: () => invoke<void>('player_prev'),
  playerSetQueue: (songs: Song[]) => invoke<void>('player_set_queue', { songs }),
  setVolume: (volume: number) => invoke<void>('set_volume', { volume }),
  setRepeat: (mode: number) => invoke<void>('set_repeat', { mode }),
  playerState: () => invoke<Record<string, unknown>>('player_state'),

  eqApply: (config: EqConfig) => invoke<void>('eq_apply', { config }),
  surroundApply: (config: SurroundConfig) => invoke<void>('surround_apply', { config }),
  spectrum: (samples: number[], sampleRate = 48000) =>
    invoke<[number, number, number]>('spectrum', { samples, sampleRate }),

  localScan: (paths: string[]) => invoke<Song[]>('local_scan', { paths }),
  localMeta: (path: string) => invoke<Song>('local_meta', { path }),
  localLyric: (path: string) => invoke<Lyric>('local_lyric', { path }),
  localCover: (path: string) => invoke<string | null>('local_cover', { path }),

  historyList: (limit: number) => invoke<Song[]>('history_list', { limit }),
  historyAdd: (song: Song) => invoke<void>('history_add', { song }),
  historyClear: () => invoke<void>('history_clear'),

  settingsGet: (key?: string) => invoke<Settings>('settings_get', { key }),
  settingsSet: (kv: Settings) => invoke<void>('settings_set', { kv }),
  fontList: () => invoke<string[]>('font_list'),
  fontImport: (path: string) => invoke<string>('font_import', { path }),

  imagePalette: (url: string) => invoke<string>('image_palette', { url }),
  imageSave: (url: string, name: string) => invoke<string>('image_save', { url, name }),
  openExternal: (target: string) => invoke<void>('open_external', { target }),
  revealPath: (path: string) => invoke<void>('reveal_path', { path }),
  resourcePath: (name: string) => invoke<string>('resource_path', { name }),

  recProbeLevels: (id: number) => invoke<string[]>('rec_probe_levels', { id }),
  recPushFrame: (rgba: Uint8Array, ptsUs: number) =>
    invoke<void>('rec_push_frame', { rgba: Array.from(rgba), ptsUs }),
  recStart: (config: RecConfig, onProgress: Channel<RecProgress>) =>
    invoke<void>('rec_start', { config, onProgress }),
  recCancel: (taskId: string) => invoke<void>('rec_cancel', { taskId }),
  recList: () => invoke<RecTask[]>('rec_list'),
  recDelete: (taskId: string) => invoke<void>('rec_delete', { taskId }),
  recClear: () => invoke<void>('rec_clear'),
  recFfmpegAvailable: () => invoke<boolean>('rec_ffmpeg_available'),

  updateCheck: (force = false) => invoke<UpdateDoc>('update_check', { force }),
  updateNotes: () => invoke<UpdateDoc>('update_notes'),
  updateInstall: () => invoke<void>('update_install'),
  fileAssocStatus: () => invoke<string[]>('file_assoc_status'),
  setAutostart: (on: boolean) => invoke<void>('set_autostart', { on }),
  setPreventSleep: (on: boolean) => invoke<void>('set_prevent_sleep', { on }),
}

/** 有序进度（下载 KB / 三段式录制 / 逐帧）一律用 Channel，广播态用 emit/listen。 */
export function progressChannel(onMessage: (p: RecProgress) => void): Channel<RecProgress> {
  const ch = new Channel<RecProgress>()
  ch.onmessage = onMessage
  return ch
}
