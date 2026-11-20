# GoMusic 电脑端开发文档（Rust + Tauri 2 + Vue 3）

**基线版本**：手机端 v26.11.36（versionCode 138）
**适用范围**：Windows / macOS / Linux 桌面客户端
**文档性质**：可交付开发评审的实现规范
**核心约束**：
- 界面**只保留横屏端**；不提供竖屏窗口、不渲染竖屏分支、不设竖屏断点。
- 与手机端横屏**逐像素对齐**，不新造布局、不新增页面。
- 手机端特有能力用「同效果最高级」方式平替。
- 业务接口、音质策略、DSP 链路、录制流程、更新文案规则与手机端保持一致。

---

## 0. 文档约定与标注

| 标注 | 含义 |
|---|---|
| 【已确认】 | 需求中给出过完整响应原文或明确行为，按原文实现 |
| 【约定】 | 按 NeteaseCloudMusicApi Enhanced 标准结构与项目既有容错解析实现，服务端以实际返回为准 |
| 【平替】 | 手机能力在桌面端的等价或更高实现 |

客户端对所有响应统一做**字段名容错**：`description / desc / briefDesc / introduction / data / result` 多形态兼容，业务码非 200 不视为成功。

**关于技术资料的引用**：Tauri 2 的 capabilities / permissions、命令调用、插件清单与文件关联配置均以官方文档为准（见 §13 参考链接）。文档中出现的 crate 版本不写死小版本，以 `Cargo.lock` 锁定为准。

---

## 1. 目标与技术选型

| 层 | 选型 | 说明 |
|---|---|---|
| 原生宿主 | **Tauri 2**（Rust，MSRV ≥ 1.90） | 系统 WebView：Windows WebView2 / macOS WKWebView / Linux WebKitGTK，**不内置 Chromium、不内置 JRE** |
| 后端 | **Rust 2021** | 网络、音频、DSP、数据库、录制编码、系统集成 |
| 前端 | **Vue 3 `<script setup lang="ts">` + Vite + TypeScript** | 全部界面、Canvas 渲染、雨雪特效、Markdown 文档 |
| 状态 | **Pinia** + vue-router 4 | 页面栈、播放态、录制进度、配置 |
| IPC | `@tauri-apps/api/core` 的 `invoke` / `Channel`；`@tauri-apps/api/event` 的 `emit/listen` | 有序进度用 `Channel`，广播状态用事件 |
| 数据库 | SQLite（`tauri-plugin-sql` / `rusqlite`） | 播放历史、录制任务快照 |
| 配置 | `tauri-plugin-store`（或 `confy` + serde_json 原子写） | 键名与手机端 SharedPreferences 一致 |
| 窗口 | 默认 **1280×800**，最小 **960×600**，`decorations: false` 自绘顶栏；**orientation 恒为 LANDSCAPE** | 不提供竖屏窗口；拉宽/缩窄只触发横屏内部重排 |
| 分发 | `tauri build` → `nsis(.exe) / msi / dmg / deb / rpm / AppImage` | 原生安装包，不交付压缩包 |

**技术选型带来的三个结构性收益**

1. **雨雪特效原样运行**：参考实现 `sn.html` 与「雨特效（css+js）」本身就是 HTML/CSS/JS。Vue 3 跑在 WebView 里，把参考文件**原样放进 `src-tauri/resources/reference/` 直接加载运行，逐字不改**，满足「严格不变、不准改变任何实现方式」。
2. **预览与成品物理同源**：录制帧由**同一个 Vue Canvas 渲染函数**产出，预览直接贴这张 canvas，编码时把同一张 canvas 的像素推给 Rust；预览与编码共用一套布局。
3. **Windows ARM64 可出原生包**：Rust 栈中 `symphonia / cpal / lofty / souvlaki` 均可编译到 `aarch64-pc-windows-msvc`；只要 FFmpeg sidecar 亦有 ARM64 构建（或走纯 Rust 编码路径）即可产出原生包。

---

## 2. 架构总览

```
┌──────────────────────────────────────────────────────────────┐
│ Vue 3 前端（系统 WebView，仅横屏）                            │
│  NavRail / AppTitleBar / PlayerWide / ControlBar(两行)         │
│  Search / Discover / Album / AlbumDesc / Artist / ArtistDesc   │
│  Equalizer / Surround / Settings / Recorder / RecordList       │
│  UpdateDoc / History / LocalMusic / Queue / Login / About      │
│  ── Canvas 单源 ── RecRenderer.ts / ArcLyrics.ts / bg/*.ts     │
│  ── 参考实现原样 ── resources/reference/sn.html, rain26.js     │
├─ Pinia stores + vue-router（页面栈） ─────────────────────────┤
├─ @tauri-apps/api/core  invoke / Channel<T> / event ───────────┤
├─ Tauri 2 capabilities（默认拒绝，按窗口授予最小权限） ─────────┤
├─ Rust commands（#[tauri::command]，AppHandle / State 注入） ───┤
│  net/     reqwest + cookie jar：§4 全量 API、降档链、风控码    │
│  audio/   symphonia 解码 → cpal 输出；队列、seek、自动续播      │
│  dsp/     EQ 双二阶 / SurroundChain / VocalReduce / FFT         │
│  local/   walkdir + lofty：Meta、内嵌歌词、封面                 │
│  db/      SQLite：history / records / cache 索引                │
│  rec/     ffmpeg（sidecar 或 ffmpeg-next）：编码 + mux           │
│  media/   souvlaki：SMTC / NowPlayingInfo / MPRIS D-Bus         │
│  sys/     tray / notification / deep-link / fileAssoc / 防休眠   │
└──────────────────────────────────────────────────────────────┘
```

**音频链路**：解码 → EQ → 3D 环绕 → 淡变/限幅 → 输出。
**录制链路**：探测音质 → 下载音频+歌词 → 人声处理 → 逐帧编码 → 音视频对齐封装。

### 2.1 IPC 命令表（Rust 侧签名 → 前端调用）

| 命令 | 主要参数 | 返回 / 通道 |
|---|---|---|
| `search` | `keywords, type, limit, offset` | `SearchResult` |
| `get_song_url` | `id, level, exact` | `SongUrl{url, level, br, size, md5}` |
| `probe_levels` | `id` | `string[]`（仅可用档位） |
| `get_lyric` | `id, force` | `Lyric`（免登录快速通道） |
| `album_desc` / `artist_desc` / `artist_detail` | `id` | 简介 / 详情结构 |
| `login` / `open_verify` | `phone, captcha\|password, verifyUrl` | `LoginState` |
| `player_play/pause/seek/next/prev` | `uri?, positionMs?` | `()`；状态走事件 `player://state` |
| `player_set_queue` / `set_volume` | `songs[], volume` | `()` |
| `eq_apply` / `surround_apply` | `EqConfig` / `SurroundConfig` | `()` |
| `local_scan` / `local_meta` / `local_lyric` | `paths[] / path` | `Song` / `Lyric` |
| `history_list` / `history_add` | `limit` / `song` | `Song[]` |
| `settings_get` / `settings_set` | `key?` / `kv` | `Settings` |
| `font_list` / `font_import` | `—` / `path` | `Font[]` |
| `image_save` / `open_external` | `url, name` / `target` | `path` |
| `rec_probe_levels` | `id` | `string[]` |
| `rec_push_frame` | `rgba: Uint8Array, ptsUs: number` | `()`（大二进制用 `tauri::ipc::Response` / 批量 Channel） |
| `rec_start` | `RecConfig` | `Channel<RecProgress>`（有序流式） |
| `rec_cancel` / `rec_list` / `rec_delete` | `taskId` | `RecTask[]` |
| `update_check` / `update_notes` | `force?` | `UpdateInfo` / `UpdateDoc` |
| `file_assoc_status` | `—` | 已注册扩展名列表 |

**约定**：自定义 `#[tauri::command]` 由 `invoke_handler(tauri::generate_handler![...])` 注册后即可调用；**插件命令必须在 capability 里显式授予权限**，否则默认拒绝。 有序进度（下载 KB、三段式录制、逐帧）一律用 `tauri::ipc::Channel`，播放态等无顺序要求的广播用 `emit/listen`。

### 2.2 关键依赖

**Rust（`src-tauri/Cargo.toml`）**

```
tauri = "2"                       tauri-build = "2"
tauri-plugin-{http,fs,dialog,notification,opener,shell,store,sql,
              updater,single-instance,deep-link,global-shortcut,
              clipboard-manager,window-state,autostart,log,cli} = "2"
reqwest (rustls-tls, cookies, gzip, brotli, json)   serde / serde_json
tokio (rt-multi-thread, sync, fs)                  thiserror / anyhow / tracing
symphonia (全格式解码)   cpal (输出)   rodio (可选)
lofty (ID3v2 USLT/SYLT, APEv2, Vorbis Comment, MP4 ©lyr)   walkdir
image (封面缩放/模糊/主色)   rustfft / realfft (2048 点频谱)
rusqlite / sqlx              souvlaki (跨三平台媒体控制)
keyring (凭据)               dirs / confy
ffmpeg-next 或 sidecar ffmpeg（录制编码与 mux）
minisign / tauri signer（更新签名）
```

**前端（`package.json`）**

```
vue@3  vue-router@4  pinia  typescript  vite
@tauri-apps/api  @tauri-apps/cli
@tauri-apps/plugin-{http,dialog,fs,notification,opener,store,sql,
                    updater,clipboard-manager,window-state,deep-link} 
markdown-it  dompurify（更新说明渲染，纯展示用）
```

不引入 UI 组件库：所有组件自写，保证与手机端横屏逐像素一致。

### 2.3 工程目录

```
gomusic-desktop/
├── index.html  package.json  vite.config.ts  tsconfig.json
├── src/                              # Vue 前端
│   ├── main.ts  App.vue  router/  stores/
│   ├── styles/tokens.css            # 设计令牌（§6.1）
│   ├── components/                  # AppTitleBar / NavRail / GoIcon /
│   │                               # SliderCard / Chip / ContextMenu /
│   │                               # ProgressBar / MarkdownDoc / Toast
│   ├── views/                       # 逐页见 §6
│   ├── canvas/
│   │   ├── RecRenderer.ts           # 唯一帧渲染源
│   │   ├── ArcLyrics.ts             # 圆弧歌词几何
│   │   └── bg/{gaussian,rain,snow}.ts
│   ├── api/{ipc.ts,types.ts}        # invoke / Channel 封装 + TS 类型
│   └── assets/fonts/                # SourceHan{Sans,Serif}-VF.otf
├── src-tauri/
│   ├── tauri.conf.json  Cargo.toml  build.rs
│   ├── capabilities/{default.json,desktop.json}
│   ├── src/{lib.rs,state.rs,commands/,net/,audio/,dsp/,local/,db/,rec/,media/,sys/}
│   ├── resources/
│   │   ├── CHANGELOG.md             # 内置更新说明（离线完整）
│   │   ├── reference/sn.html        # 飘雪参考，逐字原样
│   │   ├── reference/rain26.js      # 雨特效参考，逐字原样
│   │   └── fonts/                   # 可变字库（@font-face 由前端加载）
│   ├── binaries/ffmpeg-<target-triple>   # externalBin sidecar
│   └── icons/
└── .github/workflows/release.yml
```

### 2.4 `tauri.conf.json` 关键片段（Tauri 2 schema）

```jsonc
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "GoMusic",
  "version": "26.11.36",
  "identifier": "net.myocd.gomusic.desktop",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [{
      "label": "main", "title": "GoMusic",
      "width": 1280, "height": 800,
      "minWidth": 960, "minHeight": 600,
      "decorations": false, "resizable": true, "center": true,
      "dragDropEnabled": true
    }],
    "security": {
      "csp": "default-src 'self' tauri: asset:; img-src 'self' asset: data: blob: https:; font-src 'self' asset: data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: tauri: https://mapi.myocd.de5.net https://api.github.com; script-src 'self'",
      "capabilities": ["main-capability"]
    },
    "withGlobalTauri": false
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi", "dmg", "deb", "rpm", "appimage"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"],
    "externalBin": ["binaries/ffmpeg"],
    "fileAssociations": [
      { "ext": ["mp3"],  "mimeType": "audio/mpeg",  "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["flac"], "mimeType": "audio/flac",  "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["m4a","aac"], "mimeType": "audio/mp4", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["ape"],  "mimeType": "audio/x-ape", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["wav"],  "mimeType": "audio/wav",   "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["ogg","oga"], "mimeType": "audio/ogg", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["wma"],  "mimeType": "audio/x-ms-wma", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["aiff","aif"], "mimeType": "audio/aiff", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["dsf","dff"], "mimeType": "audio/vnd.dsd", "rank": "Alternate", "description": "GoMusic 音频" },
      { "ext": ["lrc"],  "mimeType": "text/plain", "rank": "Alternate",
        "exportedType": { "identifier": "net.myocd.gomusic.lrc", "conformsTo": ["public.plain-text"] } }
    ],
    "windows": { "nsis": { "installMode": "currentUser" }, "wix": {} },
    "macOS": { "minimumSystemVersion": "11.0" },
    "linux": { "deb": { "depends": [] } }
  },
  "plugins": {
    "updater": {
      "pubkey": "<tauri signer generate 生成的 minisign 公钥>",
      "endpoints": ["https://<发布域名>/latest.json"]
    }
  }
}
```

**文件关联说明**：`bundle.fileAssociations` 的 `ext / mimeType / rank / exportedType` 由 Tauri 写入 Windows 注册表、Linux `.desktop` MimeType、macOS `CFBundleDocumentTypes`；双击文件时路径作为命令行参数传入，用 `tauri-plugin-cli` / `tauri-plugin-single-instance` 接收并转发到已运行实例。 `rank` 用 `Alternate` 而非 `Default`：**系统默认播放器不能静默设置**，仍需用户在「打开方式」里选一次「始终」。`gomusic://` 协议由 `tauri-plugin-deep-link` 注册。

### 2.5 `capabilities/default.json`

```jsonc
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "description": "主窗口最小权限集",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging", "core:window:allow-minimize",
    "core:window:allow-toggle-maximize", "core:window:allow-close",
    "http:default", "dialog:default", "notification:default",
    "opener:default", "store:default", "sql:default",
    "clipboard-manager:default", "window-state:default",
    "updater:default", "deep-link:default", "log:default",
    "global-shortcut:allow-register",
    { "identifier": "fs:allow-read-file",  "allow": [{ "path": "$APPCONFIG/*" }, { "path": "$APPLOCALDATA/*" }, { "path": "$DOWNLOAD/*" }] },
    { "identifier": "fs:allow-write-file", "allow": [{ "path": "$APPLOCALDATA/*" }, { "path": "$DOWNLOAD/*" }] }
  ]
}
```

CSP 只有在 `app.security.csp` 显式配置时才生效；封面直链（`p1~p4.music.126.net`、`m801.music.126.net`）需在 `img-src` / `connect-src` 放行。前端 bundle 不含任何 token / 密钥，凭据与 cookie 全部留在 Rust 侧（必要时进 `keyring`）。

---

## 3. 手机能力 → 桌面最高级平替（Rust 实现）

| 手机端 | 桌面【平替】 | 实现要点 |
|---|---|---|
| Media3 / ExoPlayer | **symphonia 解码 + cpal 输出** 的自研音频引擎 | 全格式、进程内精确 seek；解码后 PCM 供 EQ / Surround 挂接；可插拔：检测到 ffmpeg sidecar 优先其解码，否则纯 Rust 解码，两条路径能力一致 |
| 前台服务 + 通知 | **TrayIconBuilder 托盘常驻** + `tauri-plugin-notification` | 播放/录制进度同时进托盘菜单与通知；窗口最小化继续跑 |
| 后台录制唤醒锁 | Windows `SetThreadExecutionState`、macOS `caffeinate`、Linux `systemd-inhibit` + 任务快照持久化 | 熄屏/合盖不中断编码；进程重启按 SQLite 快照续跑 |
| MediaSessionService | **souvlaki**：Windows SMTC / macOS `MPNowPlayingInfoCenter` + `MPRemoteCommandCenter` / Linux MPRIS D-Bus | 系统媒体键、锁屏/触控栏/蓝牙控制、正在播放信息 |
| SharedPreferences | `tauri-plugin-store`（键名与手机端一致） | 原子写；配置迁移无需改键 |
| cacheDir / filesDir | `app_local_data_dir()/cache`、`/data` | 封面/字体/歌词/音频缓存统一落此 |
| Download/GoMusic | `dirs::download_dir()/GoMusic`（可配置） | 录制成品与右键下载的大图 |
| Coil 图片加载 | `reqwest` 下载 + `image` 解码 + 三级缓存（内存/磁盘/URL） | URL 与缓存键规则不变；大二进制走 `tauri::ipc::Response` 避免 JSON 序列化 |
| Palette 主色 | `image` 缩放采样 + HSL 直方图聚类，饱和度/亮度钳制 | 用于主题强调色与封面模糊底 |
| 长按（复制/下载） | **右键上下文菜单**（复制、保存图片、打开所在目录） | 桌面自然交互；文本仍支持选中复制 |
| 系统注册默认播放器 | `bundle.fileAssociations` + `tauri-plugin-deep-link` + `tauri-plugin-cli` | 见 §2.4；首次需用户选一次「始终」 |
| MediaStore 扫描 | `walkdir` 递归指定目录 + 文件监听 | 桌面无 MediaStore，目录索引等价且更可控 |
| 摄像头挖孔 `sideInset` | 无挖孔，左导航固定 **76px**、无 inset | 横屏黑条问题桌面不存在；导航背景铺满左边缘 |
| 沉浸式状态栏 / Doze / 唤醒锁 | 不适用，设置项显示「无需设置」 | 保留设置项位置，不隐藏 |
| 易盾风控验证页 | **独立 WebviewWindow 或系统默认浏览器**打开验证链接，deep-link 回调 token | 滑块必须在真实浏览器过；系统浏览器为首选，内嵌窗口仅作兜底 |
| 内置可变字体 | 前端 `@font-face` + `font-variation-settings: 'wght' N`（100–900） | 比 Android 更原生；woff2/otf 均可；设置页与录制页共用同一份字库 |
| 逐帧 Surface / GL 特效 | **直接运行参考 HTML/CSS/JS**，Canvas2D / WebGL2 在 WebView 内 | 逐字不改；不再是「移植」而是「原样执行」 |
| 内置 `assets/CHANGELOG.md` | `src-tauri/resources/CHANGELOG.md`，`resolveResource` 读取 | 离线完整可读 |
| 应用内自动更新 | `tauri-plugin-updater`（minisign 签名）负责安装；Vue 侧仍读 GitHub Releases `body` 渲染文档 | 两条并存，互不替代 |

---

## 4. API 总表：请求方法与返回格式

> 全部由 Rust `net/` 模块用 `reqwest` 发出；前端只调 `invoke`，不直接发跨域请求（规避 WebView CORS，也避免凭据进入 JS 层）。

### 4.1 服务域名总表

| 域名 | 用途 | 登录 |
|---|---|---|
| `https://mapi.myocd.de5.net` | 网易云音乐 API Enhanced（主源） | 部分需要 |
| `https://translate.mapi.myocd.de5.net` | 歌词区间翻译 | 否 |
| `https://cloud.myocd.de5.net` | 字体/参考页静态资源 | 否 |
| `https://gomusic.myocd.de5.net` | 分享落地页 | 否 |
| `https://api.github.com` + `gh-proxy.com` | 版本更新 | 否 |
| `p1~p4.music.126.net` / `m801.music.126.net` | 封面与音频直链 | 否 |
| `https://st.music.163.com` | 风控验证页 | — |
| `https://cdn.jsdelivr.net` | opencc-js 繁简转换 | 否 |

### 4.2 网易云主源

#### 4.2.1 搜索 `GET /search` 【约定】
```
https://mapi.myocd.de5.net/search?keywords=十年&type=10&limit=20&offset=40&unblock=true&cookie=
```
| 参数 | 说明 |
|---|---|
| keywords | 关键词 |
| type | 1 单曲 / 10 专辑 / 100 歌手 / 1000 歌单 |
| limit / offset | 分页 |
| unblock | 解灰开关（通用请求强制追加） |
| cookie | 登录态（通用请求追加，歌词接口例外） |
| timestamp / randomCNIP | 通用请求防缓存追加 |

返回：
```json
{
  "result": {
    "songs": [{
      "id": 66842, "name": "十年",
      "artists": [{"id": 6452, "name": "陈奕迅"}],
      "album": {"id": 168223858, "name": "...", "picUrl": "https://p?.music.126.net/..."},
      "duration": 200000, "alias": []
    }],
    "songCount": 120, "hasMore": true
  },
  "code": 200
}
```
`type=10` 时结果在 `result.albums`（含 `id/name/artist/picUrl`）；`type=100` 在 `result.artists`。
Rust 命令：`search`；响应经 `serde_json::Value` + 字段名容错后转 `SearchResult`。

#### 4.2.2 音质取地址 `GET /song/url/v1` 【约定 + 已确认降档链】
```
https://mapi.myocd.de5.net/song/url/v1?id=66842&level=higher&unblock=true&cookie=
```
| 参数 | 说明 |
|---|---|
| id | 音乐 id |
| level | 音质档位（见 4.5） |
| unblock / cookie | 通用参数 |

返回：
```json
{
  "data": [{
    "id": 66842,
    "url": "http://m801.music.126.net/.../xxx.flac",
    "br": 960000, "size": 28481666, "md5": "",
    "level": "higher", "encodeType": "flac",
    "type": "flac", "freeTrialInfo": null
  }],
  "code": 200
}
```
- **失败判定**：网络异常 / 非 200 / 业务码异常 / `url` 为空。
- **风控终止码**：`-462`、`10004`、`460`、需登录 —— 立即终止，不逐档重试。
- 缓存必须同时存 `url` 与**实际返回档位**，不得用请求档位冒充。
- 服务端返回的 `level` 与请求不一致时按返回档位记账；录制严格模式下判为不可用（不降档）。
- Rust 命令：`get_song_url { id, level, exact }`；`exact=true` 走 `audioExactLevel`，只打一档。

#### 4.2.3 歌词 `GET /lyric` 【已确认】
```
https://mapi.myocd.de5.net/lyric?id=33894312
```
**免登录、独立快速通道**：不带 cookie、不追加 `unblock/randomCNIP/timestamp`、不重试；主请求为空才补一次带 `randomCNIP` 的（仍不带 cookie）。与 `/song/detail` 并发（`tokio::join!`），先出词再补图。

响应原文结构：
```json
{
  "sgc": false,
  "sfy": false,
  "qfy": false,
  "lrc":   { "version": 10, "lyric": "[00:00.000] 作词 : 张国祥\n[00:01.000] 作曲 : 汤小康\n[00:12.570]难以忘记初次见你\n..." },
  "klyric":{ "version": 0,  "lyric": "" },
  "tlyric":{ "version": 0,  "lyric": "" },
  "romalrc":{ "version": 0, "lyric": "" },
  "code": 200
}
```
字段：`lrc` 原文、`tlyric` 译文、`romalrc` 罗马音、`klyric` 逐字；`sgc/sfy/qfy` 为可忽略标记。解析 `[mm:ss.xxx]` 时间轴；无时间戳的纯文本按行均分补轴。歌词加载失败后点「暂无歌词」清空缓存重新拉取（`force=true`）。

#### 4.2.4 专辑简介 `GET /ugc/album/get` 【约定：仅登录】
```
curl -X GET 'https://mapi.myocd.de5.net/ugc/album/get?id=168223858'
```
未给完整响应示例，客户端按 `data.description / data.desc / data.briefDesc / description / desc / briefDesc / introduction` 容错取简介；大图取 `data.picUrl / cover / picUrl`，标题 `name`，歌手 `artist / artistName`。未登录不显示简介区，其余照常。

#### 4.2.5 歌手简介 `GET /ugc/artist/get` 【约定：仅登录】
```
curl -X GET 'https://mapi.myocd.de5.net/ugc/artist/get?id=15396'
```
同 4.2.4 容错解析，简介字段多形态兼容；未登录不显示。

#### 4.2.6 歌手详细信息 `GET /artist/desc` 【已确认：免登录】
```
https://mapi.myocd.de5.net/artist/desc?id=6452
```
响应结构：
```json
{
  "introduction": [
    { "ti": "主要成就", "txt": "获得十五座金曲奖\n两届台湾金曲奖最佳国语男歌手奖\n..." },
    { "ti": "代表作品", "txt": "音乐：七里香、青花瓷、双截棍、...\n电影：头文字D、..." },
    { "ti": "人物简介", "txt": "似乎在不经意间，周杰伦的歌成为陪伴许多人青春、成长的BGM，..." },
    { "ti": "早年经历", "txt": "..." },
    { "ti": "演艺经历", "txt": "...（超长，分段懒加载）..." },
    { "ti": "公益活动", "txt": "..." }
  ],
  "briefDesc": "周杰伦（Jay Chou），1979年1月18日出生于台湾省新北市，...",
  "count": 4,
  "topicData": [
    {
      "topic": {
        "id": 18476099, "mainTitle": "金曲奖前瞻：喜得贵子的周杰伦和他的床边故事",
        "title": "...", "content": [ { "type": 13, "id": 0, "content": "<p>...HTML...</p>" } ],
        "cover": 19106213556079276, "pubTime": 1498061009000,
        "readCount": 97353, "tags": ["金曲奖","周杰伦","流行音乐"],
        "summary": "...", "url": "/m/topic/18476099"
      },
      "creator": { "userId": 51244836, "nickname": "BiuBiu啾咪咪", "avatarUrl": "...", ... },
      "shareCount": 70, "commentCount": 182, "likedCount": 830,
      "rectanglePicUrl": "http://p4.music.126.net/...jpg",
      "coverUrl": "http://p4.music.126.net/...jpg",
      "categoryName": "幕后故事", "readCount": 97353,
      "mainTitle": "...", "title": "...", "tags": ["..."]
    }
  ],
  "code": 200
}
```
渲染规则：先 `briefDesc`，再按 `introduction[].ti` 分段展示 `txt`（`\n` 转段），超长「演艺经历」按段懒加载（Vue `v-intersect` / 虚拟列表）；`topicData` 只列 `mainTitle/title/summary/tags`，正文是 HTML 且带大量配图，不渲染正文与图片。简介与详情文字均支持右键复制整段。

#### 4.2.7 登录与风控
```
GET https://mapi.myocd.de5.net/login/cellphone?phone=13996400393&captcha=3542
GET https://mapi.myocd.de5.net/login/cellphone?phone=xxx&password=yyy
GET https://mapi.myocd.de5.net/login/cellphone?phone=...&randomCNIP=true&captcha=...
```
返回【约定】：`code`、`profile`（`userId/nickname/avatarUrl`）、`token`、`cookie`、`account`。失败码含 `-462`（风控）、`10004`、需登录、`460`、`10003`（手机号复用）。
风控验证页：
```
https://st.music.163.com/encrypt-pages?params={"sign":"...","event_id":"..."}&verifyType=40&verifyId=2001&verifyToken=...
```
**桌面用系统默认浏览器打开**（`tauri-plugin-opener`），验证完成后的 token 由 `gomusic://verify?...` deep-link 回写或粘贴回写；不内嵌滑块。

#### 4.2.8 其他已出现的主源端点 【约定】
| 方法 | URL | 用途 |
|---|---|---|
| GET | `https://mapi.myocd.de5.net/song/detail?ids=...` | 封面/时长详情（与歌词并发） |
| GET | `https://mapi.myocd.de5.net/docs` | 接口文档参考 |
| GET | `https://mapi.myocd.de5.net/` | 服务根 |

### 4.3 翻译 / 繁简 / 字体资源

| 方法 | URL | 说明 |
|---|---|---|
| POST【约定】 | `https://translate.mapi.myocd.de5.net/translate` | 歌词区间翻译；请求体 `{text, from, to}` 类，返回 `{translation / data}`，按字段容错 |
| GET | `https://cdn.jsdelivr.net/npm/opencc-js@1.4.2/dist/umd/full.min.js` | 繁简转换；桌面平替为**打包进前端的本地 JS**（Vite 依赖或内置资源），避免运行时外网依赖 |
| GET | `https://cloud.myocd.de5.net/direct/.../hei.woff2` | 思源黑体可变字库 |
| GET | `https://cloud.myocd.de5.net/direct/.../song.woff2` | 思源宋体可变字库 |
| GET | `https://cloud.myocd.de5.net/go/sn.html` | 飘雪特效参考实现（**原样内置运行**，见 §8） |
| GET | `https://github.com/Lavender-z/demo/tree/master/雨特效（css+js）` | 雨滴 GLSL 参考实现（**原样内置运行**，见 §8） |
| GET | `https://gomusic.myocd.de5.net/?s=fx_d7521a07e6548843` | 分享落地页 |
| GET | `https://api.ocd.ccwu.cc/?url=` | 备用通用代理（失败回退直连） |

**字体【平替】**：woff2 / otf 放 `src/assets/fonts/`，CSS `@font-face { font-family: 'SourceHanSansVF'; src: url(...); font-weight: 100 900; }`，运行时 `font-variation-settings: 'wght' N`。系统默认 / 思源宋体 / 思源黑体 / 导入四选一，两种内置字体共用字重滑杆，设置页与录制页共用同一份字库与配置。

### 4.4 更新与发布

```
GET https://api.github.com/repos/ike-lee-820/GoMusicForAndroid/releases?per_page=20
GET https://gh-proxy.com/https://api.github.com/repos/.../releases?per_page=20   （加速，失败回退直连）
```
返回【约定】：GitHub Releases 数组，每项含 `tag_name / name / body / assets[].browser_download_url / published_at`。
- **加速与直连两个候选都取，按长度取最完整**，候选需形似 Markdown，短于 body 90% 不采用。
- Release 正文 = 归档 `docs/release-notes/<版本>.md` 全文，**只含「修复 / 新增」两栏**，不含排错、核验、构建过程。
- 应用内 `resources/CHANGELOG.md` 与 Release 正文同源，离线完整可读。
- Vue 侧用 `markdown-it` + DOMPurify 渲染；底部**无字数统计、无「复制全文」、无「查看完整文档」行**。
- 版本号 `年.月.该月版本`（如 26.11.36），versionCode / build number 单调递增，不得回退。
- 安装由 `tauri-plugin-updater` 承担：需 `tauri signer generate` 生成 minisign 密钥对，私钥签名 release、公钥写进 `plugins.updater.pubkey`；**未签名更新会被拒绝**。

### 4.5 音质档位与降档策略【已确认】

完整档位表（`NM_LEVELS`，相邻回退，不跳档）：
```
jymaster → vivid → dolby → jyeffect → hires → lossless → exhigh → higher → standard
```
- **播放/下载**：当前档失败沿表逐档回退，每档记一行日志；命中风控立即终止；档内参数变体并发（`tokio::join!`）、档间串行；解灰匹配与游客 cookie 兜底保留。只改本次实际档位，**不改全局音质设置**。
- **录制严格模式**：`audioExactLevel` 只打所选一档；服务端返回档位不一致即判不可用，不降档、不回退、不复用别的缓存。逐档探测后的可用档位**只显示可用项**，选择不在结果里时自动切到最高可用档。

**MP4 封装限制**：FLAC / APE 不能直接写入 MP4 音轨。AAC / MP3 / AC3 / E-AC3 以压缩样本原样封装；其余格式解码转 AAC 再封装（`strength=0` 时跳过 DSP，不改音色、不降档位）。Rust 侧 ffmpeg 命令统一保留 `-c:a aac` 兜底分支，避免 `Failed to add the track to the muxer`。

### 4.6 本地音乐元数据（无网络请求）

手机端 `MediaMetadataRetriever` + 自解容器标签平替为 **`lofty` + 自研标签读取**：
- 歌名 `TITLE / title / ©nam`、歌手 `ARTIST / artist / ©ART`、专辑 `ALBUM / album / ©alb`、内嵌封面 `APIC / METADATA_BLOCK_PICTURE / covr`
- 歌词自解：**ID3v2 USLT / SYLT、APEv2 Lyrics、FLAC/Ogg Vorbis Comment `LYRICS/UNSYNCEDLYRICS`、MP4 `©lyr`**；UTF-8 / UTF-16 / GBK 自动识别
- 同名 `.lrc`（UTF-8/GBK）优先；优先级：同名 .lrc → 内嵌同步歌词 → 内嵌文本（自动补时间轴）
- SYLT 的 MPEG 帧时间按 26ms 近似换算；`content://` 场景桌面不存在，直接按文件路径读同目录 `.lrc`
- 本地歌与在线歌同一份播放历史（SQLite）；点封面/歌手**不跳转查询**、不触发在线取地址/音质探测/在线歌词

### 4.7 通用请求封装规则（`net/mod.rs`）

| 规则 | 内容 |
|---|---|
| 方法 | 除翻译外统一 `GET`；`curl -X GET` 形式 |
| 强制参数 | 通用请求追加 `unblock=true`、`cookie`、`timestamp`、`randomCNIP`（防缓存/解灰） |
| 例外 | `/lyric` 免登录快速通道：不带 cookie、不追加上述参数、不重试 |
| 超时重试 | 通用请求失败重试 2 次；歌词不重试 |
| 凭据 | `reqwest::cookie::Jar` + `keyring`；**绝不进入前端 bundle** |
| 代理 | GitHub 更新走 `gh-proxy.com` / `v6.gh-proxy.com`，失败回退直连；取最长候选 |
| 错误 | 网络异常 / 非 200 / 业务码异常 / 空数据 → `tracing` 记日志并按策略降档或提示；风控码立即终止 |
| 外部 LLM【约定】 | `openrouter.ai`、`api.sizhi.com/chat?appid=...` 历史出现但无请求体定义，作为**可插拔翻译后端**预留，不进核心播放链路 |

---

## 5. 数据模型

**Rust 侧（`serde` 派生，`#[serde(rename_all = "camelCase")]`）**

```rust
struct Song { id: i64, name: String, artist: String, artist_id: Option<i64>,
              album: String, album_id: Option<i64>, pic_url: Option<String>,
              duration_ms: i64, source: Source /* NM|LOCAL */,
              local_path: Option<String>, level: Option<String> }
struct Lyric { lines: Vec<LyricLine>, has_timeline: bool }   // {time_ms,text,trans,roma}
struct ArtistDesc { brief_desc: String, sections: Vec<Section>, topics: Vec<Topic> }
struct EqConfig { preset: String, strength: f32, bands: [Band;10], hpf: f32, lpf: f32,
                  device_calib: Device, ab_loudness: bool }
struct SurroundConfig { enabled: bool, strength: f32, scene: Scene,
                        width: f32, distance: f32, taps: Vec<u32>, reverb: f32, eco: bool }
struct RecConfig { ratio: Ratio, resolution: u32, fps: u32, level: String, bitrate: u32,
                   lyric_size: u8 /*8..96*/, info_size: u8 /*10..64*/, info_zone: f32,
                   lyric_offset_ms: i32 /*±5000*/, custom_lrc: Option<String>,
                   font_source: FontSource, wght: u16 /*100..900*/, show_trans: bool,
                   style: Style /*CLASSIC|DISC*/, background: Bg /*GAUSS|RAIN|SNOW*/,
                   vocal: Vocal /*ORIGINAL|REDUCE*/, reduce_strength: u8 }
struct RecTask { id: String, state: TaskState, phase: u8, detail: String,
                 percent: u8, output_path: Option<String>, cover_path: Option<String>,
                 config: RecConfig }
struct RecProgress { phase: u8, detail: String, percent: u8, frame: u64, total: u64 }
struct UpdateDoc { version: String, body: String }   // md，仅修复/新增
```

**前端 TS 类型由 `src/api/types.ts` 手工镜像保持同步**（`invoke<T>` 只是 TS 提示，无运行时校验）。

---

## 6. 界面设计（仅横屏 · 细致规格）

> **总量原则**：全部尺寸以手机横屏 `dp` 按 **1dp = 1 CSS px @100%** 换算；桌面 DPR 由系统缩放处理。不新增栏、不新增页面、不引入竖屏断点。窗口只有「标准横屏」与「宽屏留白」两种表现，后者仅增加外侧留白。

### 6.1 设计令牌（`src/styles/tokens.css`）

#### 6.1.1 色板

| 令牌 | 值 | 用途 |
|---|---|---|
| `--go-bg` | `#0B0D10` | 全局近黑底 |
| `--go-surface` | `#14171C` | 导航条、卡片、控制栏 |
| `--go-surface-2` | `#1C2027` | 选中态、hover 态 |
| `--go-line` | `rgba(255,255,255,.08)` | 1px 分隔线 |
| `--go-text` | `#F2F4F8` | 主文字 |
| `--go-text-2` | `rgba(242,244,248,.62)` | 副文字（歌手、说明） |
| `--go-text-3` | `rgba(242,244,248,.38)` | 三级文字（分组标题、版本） |
| `--go-accent` | 封面 HSL 主色（S 钳 0.45–0.75、L 钳 0.52–0.68） | 选中、开启态、进度已播、当前歌词 |
| `--go-accent-8` | `color-mix(in srgb, var(--go-accent) 8%, transparent)` | 选中卡片底 |
| `--go-dim` | `rgba(0,0,0,.45)` → 渐变到透明 | 封面底暗渐变（顶端 72 强度，见 §8） |
| `--go-warn` | `#E0A33E` | 增益超限提示 |
| `--go-danger` | `#E5534B` | 删除、取消、错误 |

#### 6.1.2 字体

| 用途 | 字体 | 规格 |
|---|---|---|
| UI 全局 | 思源黑体 VF | 12 / 13 / 14 / 16 / 22px，`wght` 400–600 |
| 歌词、页面大标题 | 思源宋体 VF | 歌词 8–96px（可调），标题 22px `wght 600` |
| 数字 / 时间 / 版本号 | 思源黑体 VF，`font-variant-numeric: tabular-nums` | 11–13px |
| 分组标题 | 思源黑体 VF | 11px，`wght 600`，`letter-spacing: .08em`，大写/中文不加粗 |

`font-variation-settings: 'wght' <100..900>`；导入字体同样支持可变轴时沿用同一滑杆，不支持时滑杆禁用并提示。

#### 6.1.3 间距、圆角、阴影

- 间距阶：`4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48` px
- 页面内边距 **24px**；卡片内边距 **20px**；卡片间距 **16px**；行内 gap **8px**
- 圆角：卡片 `16`、按钮 `10`、Chip `999`、封面 `12`、预览画布 `12`、右键菜单 `12`
- 阴影：`0 8px 32px rgba(0,0,0,.45)`（浮层）；卡片无阴影，用 `1px var(--go-line)` 描边
- 分隔线：`1px solid var(--go-line)`

#### 6.1.4 播放器图标（图标包 · base64 内嵌）

播放器全部控制图标取自随需求提供的图标包（`icon.txt`，实为 ZIP，解压后为 **17 枚 80×80 PNG，RGBA 透明底、纯黑线性图形**）。17 枚逐枚以 `data:image/png;base64,...` **内嵌于 §14**，运行时零外部文件依赖，不占用 `bundle.icon`（应用安装图标另按 §2.4 提供）。

| 包内文件名 | key | 用途 / 落位 |
|---|---|---|
| 继续-80x80.png | `play` | 暂停态时行 2 中央「播放」 |
| 暂停-80x80.png | `pause` | 播放态时行 2 中央「暂停」 |
| 上一首-80x80.png | `prev` | 行 2 上一首 |
| 下一首-80x80.png | `next` | 行 2 下一首 |
| 播放列表-80x80.png | `queue` | 行 2 队列、队列抽屉入口 |
| 单曲循环-80x80.png | `repeatOne` | 行 2 循环模式（态 1） |
| 列表循环-80x80.png | `repeatAll` | 行 2 循环模式（态 2） |
| 随机播放-80x80.png | `shuffle` | 行 2 循环模式（态 3） |
| 速度-80x80.png | `speed` | 倍速入口 |
| MV-80x80.png | `mv` | 行 1 MV |
| 音质-80x80.png | `quality` | 行 1 音质下拉、录制页可用音质 Chip |
| 均衡器-80x80.png | `eq` | 行 1 均衡器入口 |
| 3D环绕-80x80.png | `surround3d` | 行 1 3D 环绕入口 |
| 录制-80x80.png | `record` | 行 1 录制视频、录制列表状态角标 |
| 下载-80x80.png | `download` | 右键「下载」、简介页「保存图片」 |
| 分享-80x80.png | `share` | 右键「分享」 |
| 添加-80x80.png | `add` | 「下一首播放 / 添加到队列」 |

**渲染规格**
- 一律按 **24×24 CSS px** 显示（80×80 源图由系统 DPR 自动适配，高分屏不发虚）；按钮最小命中区 44×44、外圈 padding 10、组内 gap 8。
- 用 alpha 蒙版着色，不另备彩色图：`background: currentColor` + `mask: var(--icon) center / contain no-repeat`。默认 `color: var(--go-text)`；**开启 / 选中态 `color: var(--go-accent)`**；禁用态 `opacity: .35`。
- `GoIcon.vue` 统一收口，只认上表 key，禁止各页各画一套。图标包未覆盖的辅助图标（全屏、设置、搜索、返回、更多、收起箭头 `⟩`、窗口三按钮）沿用 **24×24 网格 / 1.7px 笔触 / round cap-join / 无填充** 的线性 SVG，视觉笔重与 PNG 图标保持一致。

#### 6.1.5 动效

| 场景 | 时长 / 曲线 |
|---|---|
| DSP 切档、预设切换 | 30ms 淡入淡出（防爆音，Rust 侧同步） |
| 通用 hover / 选中 | 180ms `cubic-bezier(.2,.8,.2,1)` |
| 页面进出 | 220ms，右进左出 + 8px 位移，不改布局 |
| 歌词切行滚动 | **420ms `easeOutCubic`**，浮点行位置连续插值 |
| 控制栏展开 / 收起 | 220ms 高度 `0 ↔ 112px` |
| 进度通知 | 跟随 `Channel<RecProgress>` 实时刷新，不做补间 |

#### 6.1.6 组件状态矩阵（所有可交互组件必须实现）

`default / hover / active(pressed) / focus-visible(2px accent 外框) / selected / disabled(opacity .35, pointer-events:none) / loading(骨架) / empty / error`。
- 按钮最小命中区 `44×44`；图标按钮外圈 padding 10。
- 滑块轨道 4px、已填 4px、拇指 12px；**鼠标按下 + 移动只改值不滚页**，滚轮在滑块上按 0.5dB / 1% 步进；拖动阈值 12px（对应手机端 `touchSlop 12dp`）。
- 所有文本节点可选中；选中后右键「复制」。

### 6.2 窗口与全局框架

```
┌─ 自绘顶栏 高 40 ────────────────────────────────────────────────┐
│ ←  →  │ 面包屑/标题（13px text-2）      │  ⌘K 搜索提示  │ ─ □ × │
├──────┬───────────────────────────────────────────────────────────┤
│ 76px │                                                           │
│ 导航 │              内容区（唯一路由出口 <router-view>）         │
│ 竖排 │                                                           │
│      │                                                           │
└──────┴───────────────────────────────────────────────────────────┘
```

- 顶栏 `data-tauri-drag-region` 可拖拽；三枚窗口按钮 40×40，hover 底 `surface-2`，关闭 hover `--go-danger`。
- 左导航 **76px 恒宽**，背景 `--go-surface` 铺满左边缘（无 inset 断层）。项目垂直居中排列，间距 8：搜索、发现、曲库、历史、录制、设置。每项 = 24px 图标 + 10px 标签（垂直排列），选中态左侧 **3px `--go-accent` 竖条** + 整块 `--go-surface-2`。底部固定版本号 `10px --go-text-3`。
- **响应式两档**：
  - `960 ≤ W < 1600`：内容区铺满剩余宽度。
  - `W ≥ 1600`：内容区最大宽 **1440px** 居中，两侧留白，**不新增任何栏**。
  - `H < 600` 不允许（窗口最小高度约束）；高度变化只影响内容区滚动，不改结构。
- **页面栈**：vue-router 全屏路由；顶栏 ← 与 `Esc` / `Alt+←` 均回上一页。
- **打开 EQ / 环绕 / 设置 / 录制 / 录制列表 / 更新说明等独立页时自动 `ctrlOpen = false`**，返回播放页恢复；全页唯一可滚容器，滑块卡片本身不滚。
- **仅横屏**：不提供竖屏播放器、竖屏功能键行、竖屏弹层、竖屏窗口尺寸、`orientation=PORTRAIT` 分支及任何 `matchMedia('(orientation: portrait)')` 代码。

### 6.2.1 键盘与右键映射（桌面增强，不改变视觉）

| 输入 | 行为 |
|---|---|
| `Space` | 播放/暂停（输入框聚焦时不拦截） |
| `← / →` | seek ∓5s；`Shift+←/→` ∓15s |
| `Ctrl/⌘+← / →` | 上一首 / 下一首 |
| `Esc` | 返回上一页；在播放页则收起控制栏 |
| `Alt+←` | 后退 |
| `Ctrl/⌘+K` | 聚焦搜索框 |
| `Ctrl/⌘+E` / `Ctrl/⌘+,` | 打开均衡器 / 设置 |
| `Ctrl/⌘+R` | 打开录制页 |
| `F` | 全屏切换 |
| 媒体键 | 由 `souvlaki` 直通 Rust 播放引擎，不受窗口焦点影响 |
| 右键 | 文本→复制；图片→保存图片 / 打开所在目录 / 复制图片；列表项→上下文菜单 |

### 6.3 横屏播放器主页 `PlayerWide.vue`（核心页）

```
┌─顶栏 40──────────────────────────────────────────────────────────┐
├────┬──────────────────────────────────────────────────────────────┤
│ 76 │ ╔═ 背景层：单张封面 cover 铺满（不拉伸/不裁小图补边）════╗   │
│ 导 │ ║ + 轻微模糊 + 顶端暗渐变 72                              ║   │
│ 航 │ ║ ┌ 左信息区 宽 34%（min 280 / max 460） ┐ ┌ 右歌词区 ┐    ║   │
│    │ ║ │ padding 40，垂直居中                  │ │  剩餘 66%│    ║   │
│    │ ║ │ ┌──────┐                             │ │          │    ║   │
│    │ ║ │ │ 96×96│ 小封面（圆 12，保留）       │ │ ArcLyrics│    ║   │
│    │ ║ │ └──────┘                             │ │  行 -3   │    ║   │
│    │ ║ │ 歌手   14px text-2  mt16            │ │  行 -2   │    ║   │
│    │ ║ │ 歌名   22px wght600 mt8             │ │ ▶ 当前行 │    ║   │
│    │ ║ │ 专辑名 13px text-3  mt4             │ │  译文行  │    ║   │
│    │ ║ │                                     │ │  行 +2   │    ║   │
│    │ ║ └─────────────────────────────────────┘ │  行 +3   │    ║   │
│    │ ║                                         └──────────┘    ║   │
│    │ ║ ┌─ 控制栏（底部，收起态仅右侧 32px 箭头 ⟩）────────────╢   │
│    │ ║ │ 展开态 高 112、左右 padding 24、圆角 16 上边          ║   │
│    │ ╚═════════════════════════════════════════════════════════╝   │
└────┴──────────────────────────────────────────────────────────────┘
```

**背景层（不可改）**
- 只有**一张** `<canvas>`/`drawImage`：专辑封面按 `cover` 等比放大铺满，**只裁边、不拉伸、不变形、不留纯色边**。
- 轻微模糊作用于这一整帧；不叠第二张封面、不叠整屏缩略糊层、不生成小图补边。
- 暗渐变从顶端 `rgba(0,0,0,.45)` 到 60% 高度透明（强度 72），保证歌词可读。
- 左信息区的 **96×96 小封面保留**，它不属于背景层。

**左信息区**
- 宽 `clamp(280px, 34%, 460px)`，垂直居中，左对齐，padding 40。
- 小封面 96×96、圆角 12、`object-fit: cover`；点击 → 专辑简介页。
- 歌手 14px `--go-text-2`；歌名 22px `wght 600` 可换行最多 2 行；专辑名 13px `--go-text-3`。
- **信息字号 10–64px 独立可调**（录制页选项，播放器页取设置默认值），与歌词字号互不影响。
- 点击歌名/专辑名 → 专辑简介页；点击歌手名 → 歌手页面；**本地歌直接 return，不跳转、不触发在线查询**。

**右歌词区 `ArcLyrics.vue`**
- 圆心位于歌词区左边缘外侧 `0.7 × 区宽`，半径 `0.78 × 区宽`；逐行绕圆心排布。
- 每行相对水平线倾斜 `20° × (行序差 / 4)`，上下至少显示 4 行，行宽 78% 区宽。
- 当前句落在弧中点（`angle = 0`）：`--go-accent` + `wght +200` + 不透明；其余按 `smoothstep(|angle|)` 淡出到 `.28`。
- 译文行距 ×1.6；长句最多 **3 行**、译文最多 **2 行**，中文按字断、英文优先空格断，实在放不下才省略号。
- 切行用**浮点行位置**在 **420ms `easeOutCubic`** 内连续滑到下一行，字号 / 透明度 / 加粗随聚焦度平滑渐变；静止时整帧复用（见 §8 速度）。
- 歌词加载失败显示「暂无歌词」，**点击重新加载**（`get_lyric { force: true }`）。
- 右键歌词 → 复制当前行 / 复制全文；区间翻译结果作为译文行显示。

**控制栏（两行展开态，不自动收起）**

```
行1 高 48px  ┌───────────────────────────────────────────────────────────┐
             │ [MV] [全屏] [均衡器] [3D环绕] [录制] │ [音质▾] [更多] [⟩] │
             └───────────────────────────────────────────────────────────┘
行2 高 48px  ┌───────────────────────────────────────────────────────────┐
             │ 00:42  ━━━━━●─────────────  04:12 │ ⏮  ▶  ⏭  ☰  🔁   │
             └───────────────────────────────────────────────────────────┘
```

- 容器：底部 16px、左右 24px、高 **112px**（行1 48 + 行2 48 + 16 gap）、`--go-surface` 92% 不透明 + `backdrop-filter: blur(20px)`、圆角 16、1px 描边。
- **两行各自 `overflow-x: auto`、隐藏滚动条**；按钮 min-width 44、gap 8；宽屏不换行、窄屏横向滚动（与手机端一致）。
- 行 1：功能按钮一律 24px 图标 + 10px padding；开启态 accent。`⟩` 为收起箭头，恒在最右。
- 行 2：进度条 `flex:1`，轨道 4px / 已播 4px / 拇指 12px；hover 显示时间气泡；点击或拖拽 seek；滚轮 ±5s；时间文本 11px tabular-nums。右侧 5 枚播放控制按钮 44×44。
- `ctrlOpen` **只认右侧箭头**：点开常驻，点收起或退出播放页才关；**无 `setTimeout` 自动收起、无「操作续期」监听**。
- 自动续播在 **Rust 音频引擎**（`STATE_ENDED → 队列下一首`），与 `ctrlOpen`、Vue 组件生命周期完全解耦；展开控制栏不影响自动切歌。单曲循环 / 队列仅一首按原逻辑不跳。
- 收起态：仅右侧 32×64 的 `⟩` 竖条，hover 展开提示。

### 6.4 独立页通用骨架

```
┌─顶栏 40： ←  页面标题（16px wght600）        [页内操作区] ─┐
├──────────────────────────────────────────────────────────────┤
│ ┌ 唯一滚动容器：padding 24，末尾 padding-bottom 32 ─────────┐ │
│ │  卡片 A（圆 16 / surface / 1px line / 内距 20）           │ │
│ │    分组标题 11px text-3                                    │ │
│ │    行 高 48，左标签 14px，右控件                           │ │
│ │  卡片 B …                                                  │ │
│ └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```
- 卡片标题 14px `wght 600`，右上可放「重置 / 恢复默认 / A·B」文字按钮（13px accent）。
- 行分隔 `1px --go-line`；行内控件右对齐，`max-width: 360px`。
- 滑块卡片本身 `overflow: hidden`，`pointerdown` 消费纵向 wheel/drag，父容器不拦截（对应手机端 `nestedScroll` 吃掉 `available.y`）。
- 空态：居中 24px 图标 + 14px 说明 + 13px accent 行动按钮；错误态：warn/danger 文案 + 「重试」。

### 6.5 搜索 / 发现 / 新碟 / 榜单 `Search.vue` `Discover.vue`

```
┌ 顶部 搜索框 高 44（圆 10，左侧 20px 放大镜，占位「搜索歌曲 / 专辑 / 歌手」） ┐
│  右侧：热词横向 Chip 行（数据源：网易云主源）                               │
├ 标签行 高 40： 单曲 | 专辑 | 歌手 | 歌单   （选中态 2px accent 下划线）     │
├ 列表（虚拟滚动，行高 64）                                                   │
│  # │ 封面 44×44 圆8 │ 标题 14px / 副标题 12px text-2 │ 音质徽标 │ ⋯ │    │
└──────────────────────────────────────────────────────────────────────────┘
```
- 发现页四块卡片：新碟（横向封面卡 120×120）、新歌、榜单、热词；数据由 Rust `net/` 从主源获取，端点以 §4 为准。
- 列表项 hover 底 `--go-surface-2`；右键菜单「播放 / 下一首播放 / 复制名称 / 查看专辑 / 查看歌手 / 下载」。
- 分页：滚动到底自动加载 `offset += limit`；加载中行尾骨架；无更多显示 12px text-3 提示。
- 双击行 = 播放；单击 = 选中（不跳转）。

### 6.6 专辑页 → 专辑简介页

**专辑页**
```
┌ 左：封面 200×200 圆16 │ 右：专辑名 22px、歌手 14px accent、曲目数/年份 12px ┐
├ 曲目列表（行高 56：序号 / 歌名 / 时长 / 音质徽标 / ⋯） ───────────────────┤
└──────────────────────────────────────────────────────────────────────────┘
```
**专辑简介页**（点击封面 / 标题进入）
```
┌ 左 40%：专辑大图（最大 360×360，圆 16，contain 不拉伸） ───────────────┐
│  下方：「保存图片」按钮 + 右键菜单（保存 / 打开所在目录 / 复制图片）        │
├ 右 60%：标题 24px wght600 / 歌手 14px accent（可点击 → 歌手页面）        │
│         简介卡片（仅登录时显示）：正文 14px，行高 1.8，可选中、右键复制整段│
│         未登录：显示 13px text-3「登录后查看专辑简介」+「去登录」          │
└──────────────────────────────────────────────────────────────────────────┘
```
- `getAlbumDesc(id)` → `/ugc/album/get?id=`；保存目录 `~/Downloads/GoMusic/图片/`。
- 本地按名字分组的专辑页优先用 `album_id`，没有才严格按名字搜；本地项不触发在线查询。

### 6.7 歌手页 → 歌手简介页

**歌手页**：顶部大图（宽 100% 高 220，cover + 暗渐变）+ 名字 24px + 热门歌曲列表 + 专辑横向卡片。点击**大图或名字**进入简介页。

**歌手简介页**
```
┌ 左 32%：歌手大图（最大 320×320，圆 16，contain）+ 右键保存 ────────────┐
├ 右 68%（唯一滚动容器）────────────────────────────────────────────────┤
│  名字 26px wght600                                                      │
│  ── 简介（/ugc/artist/get，仅登录；未登录显示占位）──────────────────    │
│  ── 详细信息（/artist/desc，免登录）─────────────────────────────────    │
│     briefDesc：14px / 行高 1.8 卡片                                     │
│     introduction[]：每段 = 13px accent 小标题 + 14px 正文，\n 转段      │
│        超长「演艺经历」按段懒加载（IntersectionObserver）                 │
│     topicData：仅标题 / 摘要 / 标签 Chip，不渲染 HTML 正文与配图          │
└────────────────────────────────────────────────────────────────────────┘
```
所有文字右键可复制整段。

### 6.8 均衡器页 `Equalizer.vue`（独立全屏页）

```
顶栏： ← 均衡器                        [A·B 对比] [重置] [恢复默认]
┌ 卡片：预设 ────────────────────────────────────────────────────────┐
│ 分组标题「人声」   ┌ 纯净人声 ─────────────┐                        │
│                    │ 人声更靠前、齿音不刺耳 │  [开关]               │
│                    └───────────────────────┘  高 88，3 列网格       │
│ 分组「音乐」 澎湃乐场 / 沉重低音 / 温暖圆润 / 冷冽明亮               │
│ 分组「空间」 缥缈音场 / 现场环绕                                     │
│ 分组「创意」 复古音效（电话 / 老式收音机）  ⚠ 创意·默认关闭          │
│ 分组「夜间」 深夜聆听（动态等响度，标注「动态功能，非固定预设」）     │
├ 卡片：强度  0 ━━━━━●━━━━━ 100%   右侧数值 + 单位，步进 1% ──────────┤
├ 卡片：高级（默认折叠，点击展开）────────────────────────────────────┤
│  10 段竖向推子（31/62/125/250/500/1k/2k/4k/8k/16k Hz）              │
│   每段：频率标签 11px / 推子高 160 / 数值 ±12dB（0.5dB 步进）        │
│  Q 值 0.3–8 · HPF 20–200Hz · LPF 4k–20k                             │
│  ⚠ 低频>4.5dB / 中频>6dB / 高频>3dB → 卡片内 warn 行                │
├ 卡片：设备校准  [耳机] [音箱] [车载]  三 Chip ───────────────────────┤
└────────────────────────────────────────────────────────────────────┘
```
- 预设卡：名称 14px wght600、一句话听感 11px text-2、右下开关；选中态 1px accent 边框 + `--go-accent-8` 底。
- 共 **9 预设 + 老式收音机变体**；创意组单独一块并带标签，默认关闭，切换前 30ms 淡变。
- A/B 必须响度匹配；等响度不与固定 EQ 默认叠加；全部参数可按设备校准，不硬编码唯一值。
- `eq_apply` 把配置送 Rust 双二阶链（解码后 PCM），不是系统 5 段 EQ。

### 6.9 3D 环绕页 `Surround.vue`

结构与均衡器页同骨架：
- 默认层：总开关、**强度 0–100%**、场景三 Chip（耳机 / 音箱 / 车载）。
- 高级层（折叠）：声场宽度、距离、反射抽头、混响时间、省电档开关。
- 顶栏：A/B、重置、恢复默认。
- 底部 13px text-3 说明「独立 DSP，不使用均衡器；150Hz 以下不空间化」。
- DSP：交叉馈送 + 对侧耳 ITD≈0.25ms + 高频滚降；质数抽头早期反射 7/11/19/29/37ms + 轻量反馈混响；输出软限幅 −1dBFS；30ms 淡入淡出。
- 入口在控制栏第一行，图标与全组同规格；开启态 accent。

### 6.10 设置页 `Settings.vue`

分组卡片，唯一滚动容器，行高 48：
| 分组 | 行 |
|---|---|
| 播放 | 后台保活（托盘常驻）、默认音质档位（下拉 9 档）、自动降档策略 |
| 网络 | 代理（gh-proxy / 直连）、风控登录 / 验证（系统浏览器打开）、查看日志 |
| 外观 | 字体（系统默认 / 思源宋体 / **思源黑体** / 导入）四 Radio、字重滑杆 100–900 + 一行实时样例 |
| 内容 | 播放历史、录制列表入口、检查更新、查看完整更新说明 |
| 桌面专属 | 文件关联状态与注册、媒体键桥接开关、防休眠开关、开机自启 |
| 不适用项 | 沉浸式状态栏 / Doze / 唤醒锁 → 灰字「无需设置」+ disabled |

- 字体选中即时全局生效（CSS 变量 `--go-font-ui` / `--go-font-lyric` 切换），不需重启；CFF2 可变字体在个别 WebView 解析失败时自动回退系统字体并 Toast 提示。
- 「检查更新」调 `tauri-plugin-updater`；「查看完整更新说明」进 §6.13 页面。

### 6.11 录制页 `Recorder.vue`（独立全屏页，所见即所得）

```
┌ 顶栏： ← 录制视频                                  [开始生成] ───────┐
├──────────────────────────┬───────────────────────────────────────────┤
│ 左：预览区（flex 1）     │ 右：配置区（宽 420，唯一滚动，padding 24） │
│  ┌ 画布：按 ratio fit 居 │  ① 可用音质  [320k][FLAC][Hi-Res] Chip 行  │
│  │  中，最大占页高 34%、 │     仅显示探测通过项，自动选最高可用         │
│  │  ≤240px，圆 12、描边  │  ② 画面 比例/分辨率/帧率/码率（各一行）      │
│  │ ┌──────────────────┐ │  ③ 文字 歌词字号 8–96 / 信息字号 10–64      │
│  │ │ 单张封面 cover 铺 │ │     信息区大小 / 字体四选 + wght 滑杆        │
│  │ │ 满 + 轻模糊 + 背  │ │     译文开关 / 偏移 ±5000ms / 粘贴 LRC(重置)│
│  │ │ 景叠层(高斯|雨|雪)│ │  ④ 样式  [经典] [圆盘]                       │
│  │ │ 左：小封面+三行字  │ │  ⑤ 背景  [高斯模糊] [雨季] [雪天]            │
│  │ │ 右：经典直列 /     │ │  ⑥ 人声  [原声] [减弱] + 强度 0–100%        │
│  │ │      圆盘 ArcLyrics│ │                                               │
│  │ │ 底部进度线         │ │  ── 生成进度（开始后显示）────────────────  │
│  │ └──────────────────┘ │  ① 下载音频+歌词  ▓▓▓▓░ 68%  1.2MB/1.8MB   │
│  │ 说明：预览固定 =      │  ② 人声处理      ▓▓▓▓▓ 100% 原声直封装     │
│  │ duration/2 那一帧     │  ③ 录制视频      ▓▓░░░ 42%  帧 1210/2880   │
│  └──────────────────────┘  [取消]                                      │
└──────────────────────────┴───────────────────────────────────────────┘
```

**流程固定四步**：
```
① 识别可用音质 → 逐档探测（exact），仅显示可用档位；当前选择不在结果里自动切最高可用
② 展示预览   → 固定渲染「歌曲时长一半」那一帧，按比例 fit 居中，不占满屏幕
③ 配置选项   → 全部选项即时反映到预览（防抖 120ms 重渲染）
④ 开始生成   → 三段式详细进度（页面 / 托盘通知 / 录制列表三处同步）
```

**配置项完整清单**
- 比例 `16:9 / 9:16 / 1:1 / 4:3`；分辨率 480–1440p；帧率；码率
- 音质（**仅可用档位**）；歌词字号 **8–96**；信息字号 **10–64**；歌曲信息区大小
- 歌词全体快进/延迟 **±5000ms**；粘贴 LRC 改词（可重置）；译文开关
- 字体：系统默认 / 内置思源宋体 / 内置思源黑体 / 导入；可变字体 `wght` 100–900 滑杆
- 人声：原声 / 减弱（强度 0–100%，100% 约等于伴奏）
- 样式：**经典**（左信息 + 右直列歌词）/ **圆盘**（= 横屏播放器圆弧歌词模式，复刻 `WidePlayerPane + ArcLyrics` 几何，非黑胶唱片）
- 背景：**高斯模糊 / 雨季 / 雪天**三种，统一以**一张铺满不拉伸的专辑封面**打底

**预览 = 成品（结构性保证）**：页面上的 `<canvas>` 就是 `RecRenderer.ts` 的输出；录制时逐帧把**同一张 canvas** 的像素（`getImageData` → `Uint8Array`）经 `rec_push_frame` / `Channel` 推给 Rust ffmpeg 编码。不存在第二套预览布局。

**三段式进度**：`Channel<RecProgress>` 推送 `{phase, detail, percent, frame, total}`，Pinia 同步到页面、托盘通知（`setProgress`）、录制列表。
1. 下载对应音质音频与 `/lyric` 歌词（按 KB 刷新）
2. 按选择做原声直封装或减弱人声处理
3. 逐帧录制视频，显示当前帧 / 总帧数

**音画对齐**：音频首样本与视频首帧统一映射虚拟时间轴 `0`；成品时长以音频真实时长为准。
**后台**：防休眠 + 托盘进度通知 + 取消按钮 + SQLite 任务快照续跑；最小化继续。
**进入录制页暂停播放、退出恢复**（Rust 播放引擎直接 pause/play 兜底，按「进入前是否在播」判定；Activity 被强制回收不触发恢复）。

### 6.12 录制列表页 `RecordList.vue`（设置内）

- 进行中置顶：卡片显示封面缩略 48×48、歌名、阶段标签、进度条、百分比、详情行、「取消」。
- 历史按时间倒序：状态 Chip（完成 / 失败 / 已取消）、成品时长与大小、「打开成品」/「打开所在目录」/「删除」。
- 顶栏「清空历史」（二次确认 Dialog）。
- 与托盘通知、录制页读同一份 `RecTask[]`。

### 6.13 更新说明页 `UpdateDoc.vue`

```
┌ 顶栏： ← 更新说明                                    ┐
┌ 说明卡片（圆 16，内距 20）─────────────────────────┐
│ 版本 26.11.36 · 发布日期                            │
│ [markdown-it 渲染区]                                │
│   修复                                              │
│   · …                                               │
│   新增                                              │
│   · …                                               │
│                              右下角 [重新载入] 13px │
└────────────────────────────────────────────────────┘
```
- Markdown 语法覆盖：标题 / 列表 / 表格 / 加粗 / 行内代码 / 代码块 / 引用 / 分隔线；HTML / 图片 / 链接按纯文本显示（经 DOMPurify）。
- 联网读 `/releases` 的 `body`，断网读 `resources/CHANGELOG.md`，**离线也完整**；加速/直连取最完整候选（≥ body 90% 且形似 md 才采用）。
- 滚动区底部 padding 32，「重新载入」不遮正文；载入中显示「载入中…」并禁用。
- **底部无字数统计、无「复制全文」、无「查看完整文档」行**；正文在可选中容器里支持右键/选中复制。
- 对外文案只有「修复 / 新增」两栏。

### 6.14 播放历史 / 本地音乐 / 队列 / 登录 / 关于

| 页 | 规格 |
|---|---|
| 播放历史 | 行高 64 虚拟列表，在线 + 本地统一；封面 44、标题、来源 Chip（在线 / 本地）、播放时间；右键「播放 / 从历史移除」 |
| 本地音乐 | 拖放音频文件到窗口（`dragDropEnabled`）或「打开方式」送入 → `local_meta` 解析 → 入历史 → 直接按路径播放；**点封面/歌手不跳转**；右键「打开所在目录 / 重新读取元数据」 |
| 播放队列 | 右侧抽屉 360px，行高 56，拖拽排序，当前行 accent 竖条；与托盘菜单「下一首播放」联动 |
| 登录 / 验证 | 卡片宽 420 居中：手机号、验证码/密码、登录按钮；风控时用系统浏览器打开验证页，deep-link 回写 token；失败码文案中文 |
| 关于 | 居中Logo 64、版本 26.11.36、构建号、更新说明入口、GitHub 链接（opener 打开）、凭据轮换提示 |

### 6.15 浮层、托盘与通知

- **右键菜单**：宽 200、圆 12、`--go-surface-2`、1px line、阴影；行高 36、icon 16、字号 13；hover `--go-surface`；边界自动翻转。
- **Toast**：底部居中、圆 10、13px、2.5s 自动消失；错误用 danger 左边条。
- **Dialog**：宽 420、圆 20、标题 16px、正文 14px、按钮右对齐「取消 / 确认」。
- **托盘**：常驻图标 + 菜单「显示主窗口 / 播放·暂停 / 上一首 / 下一首 / 当前播放：… / 录制进度 xx% / 退出」；录制时图标叠加进度角标。
- **通知**：`tauri-plugin-notification`，标题「GoMusic 录制中」，正文阶段+详情+百分比，带「取消」动作。

### 6.16 空 / 加载 / 错误 / 权限态

| 状态 | 表现 |
|---|---|
| 加载 | 列表 3 行骨架（64px 高、1s 微光）；预览区 12px「载入中…」 |
| 空 | 24px 线性图标 + 14px 说明 + 13px accent 行动按钮（如「去搜索」「重新载入」） |
| 无歌词 | 歌词区居中「暂无歌词」，**可点击重新加载** |
| 未登录 | 简介区占位「登录后查看」+「去登录」，其余内容照常显示 |
| 风控 | 中文提示 + 「打开验证页」按钮，不显示英文堆栈 |
| 音质不可用 | 明确报错「所选档位不可用，请换档」，不静默降级 |
| 权限拒绝 | 文件/目录授权失败时提示并给「重新选择」，不崩溃 |

---

## 7. 音频链路与 DSP（Rust）

```
symphonia 解码（或 ffmpeg sidecar）
  → ChannelChain  EQ 10 段双二阶（gain/Q/HPF/LPF，仅开启时）
  → SurroundChain 3D 环绕（独立，不依赖 EQ；150Hz 以下旁路）
  → 淡变(30ms) / 限幅(−1dBFS)
  → cpal 输出
```
- **内核可插拔**：检测到 ffmpeg sidecar 优先其解码；未带时纯 Rust `symphonia` 接管，两条路径播放能力一致。
- **人声减弱**：显式小端 PCM、按解码器真实 `sampleRate/channelCount/位深` 解析（float PCM 转 16bit）、单声道中频衰减、标准 karaoke 矩阵 `L−k·R / R−k·L`（100% 抵消中置并回补低频）、尾部 drain 到 EOS。纯信号处理，无 AI / 模型；单声道老录音与复杂古典效果有限。
- 2DFT / Adaptive REPET 参考实现（`dsp/repet.rs`）默认不接构建；接入时按「单声道 2DFT、立体声规整曲目 Adaptive REPET、弱重复性给出质量提示」选择，NEON 在 `aarch64` 上用 `std::arch` 或 `wide` 加速，跨 JNI 零拷贝问题在 Rust 内不存在（同一进程内存）。
- 强度调整统一 0.5dB 步进；A/B 保持响度一致；频谱分析用 `rustfft` 2048 点 + 0.78 平滑（供雪天音频响应）。

---

## 8. 特效参考实现：原样运行

**背景总规则**：一张专辑封面 **cover 等比放大铺满**，只裁边、不拉伸、不变形、不留纯色边；整层轻微模糊只作用于这一张；不叠第二张封面、不叠整屏缩略糊层、不用放大小图补齐。左侧信息栏的小封面**保留**，它不属于背景层。

**实现前提**：参考实现本身即浏览器代码，可在 WebView 中直接执行。

| 模式 | 桌面实现 |
|---|---|
| 高斯模糊 | `bg/gaussian.ts`：对单张封面帧做一次轻微模糊（Canvas `filter: blur()` 或降采样盒式近似），保留轮廓 |
| 雨季 | **直接加载 `resources/reference/rain26.js` 原文件**在 WebGL2 中运行：`shader.fragment`、四层 `Layer`、N12 噪声、32 次采样、40×40 网格、全部常量**逐字不改**；GLSL 显式 `highp`；`uTime` 每渲染一帧 +1；`dispImage` 上传的就是已按比例做好的封面底；读回空帧/纯色板时保留封面底，绝不整屏替换；`glClear` + `glFinish` 兜底。误判修正：封面大面积米色不得被「5×5 亮度差 < 4」当作纯色丢弃 |
| 雪天 | **直接加载 `resources/reference/sn.html` 原文件**（或把其 `<script>` 原样挂到录制 canvas 的透明叠加层）：三层精灵 r 3.6/2.1/1.1、speed 78/48/26、alpha .95/.72/.42、swayAmp 22/14/8、drift 100/74/50、spawn .18/.32/.50、径向渐变四段色标、`reset/update/render`、`drive()` 的 breathe 与 clamp、三路 lerp 0.28/0.20/0.16 **一字不改**；Web Audio `AnalyserNode` 在桌面可直接用（浏览器原生），或从 Rust 侧 `rustfft` 推 `{bass,mid,high}` 保持与录制帧同步 |

- 背景模式仅三种：高斯模糊 / 雨季 / 雪天；读取到历史配置中的其他取值时回退「高斯模糊」。
- **速度**：静态层预渲染一次；每帧算画面签名（高亮行、进度条每 2px、样式/背景/字号/字体/偏移），相同直接复用上一帧；雨雪逐帧部分照常绘制。日志输出「帧渲染 N 帧，复用 M 帧（X%）」。
- **参考文件只读**：`resources/reference/` 下文件纳入 CI 校验和比对，禁止任何格式化或改写；若上游更新，整文件替换并重新比对。

---

## 9. 持久化与文件布局

```
<app_local_data_dir>/
  store.json               全局设置（键名对齐手机端 SharedPreferences）
  cache/                   封面、字体、歌词、音频缓存（键含实际档位）
  data/
    gomusic.db             SQLite：播放历史（在线+本地统一）、录制任务与快照
  resources/
    CHANGELOG.md           内置更新说明
    reference/sn.html      飘雪参考（只读）
    reference/rain26.js    雨特效参考（只读）
    fonts/
      SourceHanSans-VF.otf   可变思源黑体 wght 100–900
      SourceHanSerif-VF.otf  可变思源宋体
<download_dir>/GoMusic/
  图片/                    右键保存的大图
  影片/                    录制成品 MP4
```

---

## 10. 构建与打包

- 命令：`npm run tauri dev` / `npm run tauri build`；`beforeBuildCommand = npm run build`，`frontendDist = ../dist`（Vite）。
- 产物：`GoMusic-<版本>-windows-x64-setup.exe`（NSIS）、`.msi`（WiX）、`windows-arm64` 原生包、`macos-x64.dmg` / `macos-arm64.dmg`（或 `universal`）、`linux-amd64.deb/rpm/AppImage`、`linux-arm64.deb`。
- **签名不是可选项**：macOS Developer ID 签名 + notarize（否则 Gatekeeper 拦截）；Windows Authenticode 签名（否则 SmartScreen 警告）；Linux 附 checksums。 更新包另需 minisign 密钥对。
- `externalBin` 放各 target triple 的 ffmpeg；CI 用 matrix 分别产出，不假设单一二进制可跨平台。
- 平台特定配置放 `tauri.windows.conf.json` / `tauri.macos.conf.json` / `tauri.linux.conf.json`，与主配置合并。
- CI 矩阵：`windows-latest`（x64）、`windows-11-arm`（arm64，若 runner 可用）、`macos-latest`（x64 + arm64）、`ubuntu-24.04`（amd64）、`ubuntu-24.04-arm`（arm64）；Linux arm64 包用 `dpkg-deb -f` 断言 `Architecture=arm64` 才放行。
- 打包脚本不得用 BSD `find -maxdepth`、不得假设 `python3` 存在；全部交给 `tauri build` 原生流程。
- 桌面端仓库与手机端分仓，版本号各自独立；桌面不复用手机端 tag。
- 元数据（`productName`、安装包描述）保持纯 ASCII 或显式 UTF-8，避免 Windows 平台默认编码读参出错。

---

## 11. 验收清单

1. 窗口恒横屏，无竖屏布局、无竖屏入口；左导航 76px 无黑条、无 inset 背景断层；顶栏可拖拽、三按钮可用。
2. 控制栏两行展开后不自动收起，播放到曲末自动切歌；两行各自横向滚动可用。
3. 全部 API 走 §4 封装；`/lyric` 请求不带 cookie；风控码立即终止降档；凭据不出现在前端 bundle。
4. capabilities 为最小权限集，插件命令未被授予时确实被拒绝；CSP 生效且封面直链可加载。
5. 录制严格音质：探测后只列可用档；服务端降档返回即判不可用；原声 AAC 原样封装、FLAC 转 AAC 不报 `Failed to add the track to the muxer`。
6. **录制预览的 canvas 与送编码的像素来自同一个 `RecRenderer.ts`**；背景一张 cover 铺满不拉伸；信息栏小封面存在；雨雪叠在封面上不整屏替换；`resources/reference/` 文件校验和与上游一致。
7. 三段式进度在页面、托盘通知、录制列表三处一致；最小化/防休眠下能跑完；开头音画对齐、时长以音频为准。
8. 设置可切换**思源黑体**并调字重；更新说明离线完整、只有「修复 / 新增」两栏、无字数统计与复制行。
9. 文件关联可把音频送进应用；内嵌 USLT/SYLT/APEv2/Vorbis/©lyr 歌词能读出并滚动；点封面/歌手不跳转。
10. Release 正文与归档 md、内置 CHANGELOG 逐字一致；版本号连续、build number 递增；updater 只接受签名更新。
11. 媒体键在窗口失焦时仍可用（SMTC / NowPlaying / MPRIS）。

---

## 12. 已知限制与风险

- 中置抵消是纯信号处理近似伴奏，不承诺录音室级分离；复杂古典 / 自由爵士 / 单声道老录音效果有限。
- 雨雪特效为**参考文件原样运行**，仍需跨 WebView（WebView2 / WKWebView / WebKitGTK）做一次实机画面验收；`sn.html` 若更新需整文件替换并重新比对校验和。
- 严格音质模式下所选档位不可用会**明确报错**而非静默降级，需要低档成品须在界面换档。
- Windows ARM64 原生包的前提是 ffmpeg sidecar 亦有 `aarch64-pc-windows-msvc` 构建；否则退回纯 Rust 编码路径或仍以 x64 仿真分发。
- 系统默认播放器不能静默设置，`rank` 固定 `Alternate`，需用户在「打开方式」里选一次「始终」。
- 进程被强制终止时，录制按 SQLite 快照续跑；非正常退出不触发「恢复播放」。
- 外部 LLM 翻译端点无请求体定义，作为可插拔后端预留，不进核心链路。
- 发布凭据属敏感信息，须轮换并只存于 CI Secrets / `keyring`，不写入仓库与文档；前端 bundle 视为不可信，任何 token 不得进 JS。

---

## 13. 参考链接

- Tauri 2 Capabilities / Permissions：https://v2.tauri.app/zh-cn/security/capabilities/ 
- 从前端调用 Rust（`invoke` / `@tauri-apps/api/core`）：https://tauri.app/zh-cn/develop/calling-rust 
- Tauri 官方插件清单：https://v2.tauri.org.cn/plugin/ 
- Tauri 2 + Vite 前端配置：http://v2.tauri.org.cn/start/frontend/vite 
- 文件关联配置（`bundle.fileAssociations`）：Tauri 官方 `examples/file-associations` 及社区说明 
- Windows NSIS / WiX 安装器配置：https://codershandbook.com/tauri-20-windows-build-creating-exe-and-msi-installers 

---

*本文档基于手机端 v26.11.36 全部已实现行为整理；标注【约定】的响应结构以服务端实际返回为准，客户端按字段名容错。Tauri 2 具体字段以项目锁定的官方 schema 为准。*

---

## 14. 播放器图标资源（base64 内嵌）

以下即 `src/assets/icons.ts` 的完整内容，由图标包 **17 枚 PNG 逐枚生成、全部内嵌**，粘贴即可用；`GoIcon.vue` 通过 `PLAYER_ICONS[key]` 取出 data URI 写入 CSS 变量 `--icon`。

```ts
// src/assets/icons.ts —— 图标包：17 × 80×80 PNG，RGBA 透明底
export const PLAYER_ICONS = {
  play: 'data:image/png;base64,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',
  pause: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAACOElEQVR4Aeycv05UURyEV42JhYWdPoS1jYWlhb6GnZXGytbKwsaW14CEloQGWt4B6CgoIPyd2WRLQvHde4DsR85wTi6Z+d3z7WzL84U/iIAAEb7FQoAChASg3QYKEBKAdhsoQEgA2m2gACEBaLeBAoQEoH1kA1/lXT9FP6Nv0btoqtWsZja7MzrrruxJn48C2Dkf8+b/oz/R3+hf9DqiqxnNamazO6OzOpNm3+sfMiRv0UZ8zf4+6vlN9s/Rh4iuZjSrmc3ujC8J7TnbvGsUwBe5xtvoWbRaL3Pos2xoNaNZq5DO6Fe6M1fPZttHAZztAg8dLED4CQhQgJAAtNtAAUIC0G4DBQgJQLsNXA+A8JYz2m0ghCtAAUIC0G4DBQgJQLsNFCAkAO02UICQALTbQAFCAtBuAx8tQPhiT8VuA+EnJUABQgLQbgMFCAlAuw0UICQA7TZQgJAAtNtAAUIC0G4DpwQIs9bSbgPhxy5AAUIC0G4DBQgJQLsNFCAkAO02UICQALTbQAFCAtC+9g2E/PzvbU8F4FVe9Ci6iVbrIoc+y4bWcdzNyrZcnXGY02U0+xr1FT7LTTajg6jnk+zb0X5E114CmtXMZnfGVp6dR7OvUQCvc5Pd6Hv0O/oV/YhOI7qa0axmNrszOqszafa9/lEA+yJtx04O/RclG9mn+PomZrma1cxmd0ZnLf8w96+RAOe+y4PkCxBiF6AAIQFot4EChASg3QauIUB45WntNhDyFKAAIQFovwUAAP//gPOlOwAAAAZJREFUAwCLaTChtasPdgAAAABJRU5ErkJggg==',
  prev: 'data:image/png;base64,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',
  next: 'data:image/png;base64,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',
  queue: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAAEgUlEQVR4AeycS8hNURTHL1HIKwbMmJjJ4/OaKSZMTDAX8ioJA0XeRBlQUgZCJgxkZCATyhAJmZkwY0CeUYT/j2/gdPa55+61fJ3ju0vrb9291ll37/O7+77WPX0jO/HPRSAAuvB1OgEwADoJOMtjBwZAJwFneezAAOgk4CyPHRgAnQSc5bEDA6CTgLO8H3egE1mxPAAWeWSPAmA2smJBACzyyB4FwGxkxYIAWOSRPQqA2ciKBQGwyCN7ZAVI3XLN9lj6Jv2U8IyJk1coaeTGKTO+JRqjdZiNk7EUz1TRRWmuNErC8IyJzyCQEItdpvhx6VwLdFZr2CvNlkxmBThHs02RUkYckKncEgXPSDuldS3Qeq3hoMSDOV0+26wAx2qmEVLKiJNP5RYoOEviGLnWGA84yl6QFWD2RIMFH+W/Sm2zH1rQGynbrABfaybeNORKRvxVKfoncFPulvRW+tS7OkN1LA8oazmvtTyUss0K8L5mui29k3gHluvgGRN/QCAhwG5WfK20vQXaqjWskPZLJrMCZEfs1ox7pEPSEQnPmDh5hZJG7q4yV1qgq1qDaeep7rdZAVLMbrqgG8ekwxKeMXEN+8M8APuDUM1ZBsAaQHXpAFhHqCYfAGsA1aUDYB2hmnwArAFUl/YCpLvyd1uKcd2cwyrvAUgLiFYQLSG6GXjGxIcVpG4nYwVI6wdotIJoCdGawjMmTj41L12Y+UrskPjw3bQOaB2bpKr1KtXdrAAHdLfzpJQRH+ikMp0OLaMTSp2U+OrXtPgKekprOS3xUiSXZ1aAkzRNVS1x8jqkZEsVQbT0dbNx4xkxWaugobBYPts42ewiZwFdG+ddDEm5aV1WgO91CjQh5UpGnHwpocC9QX2Wb4MBra4F13WdVoCPdK/8AidXMuLkSwkFnkj7BsXrT9PiNbiXFpyWnDYrQFpWNESP6m6vSTckPGPi5BUqGY84gPnI0/Q7MPO7W3BWgJB5pv94N90iv0HCMyauYX+YByCE+IGI3xU+aIBnrJv9Y16A/UOq4kwDYAWYXsMBsFdSFccFwAowvYYDYK+kKo4rAKw4JsJdCHgA0gKiFURLiA+leMbEu0w5vFJWgLR+aAHRCuLrGF+J8IyJk68iRY6LMNfogKa1WmtYKJnNCpDWDy0gWkG0hFgAnjHxRQQSYndy9cJ15S61QJe1Bq7l4YJP3cw3K8Bpmmq0lDLigErlVim4UuIizInybRBr2aa1mHaiFaDmM9kEVbXxhyc4TNXaso3C7CIVfJHorMiVjDj5UkIBroR6Ls8xcq0x2mwoe0FWgE81ExcmypWMeNViuK5wlyq4TpoWWNPqpQWn5VabFeAL3eVGid7ed3kMz5jW1ksCCdGt4dpALmjkuKblbsFZAdK2vyNA/ETJmwbvwHjGACKvdNLI8RSnBda03C04K8AkmX4MBkDno973AJ384q+3BUAvAWd9PIUDoJOAszx2YAB0EnCWxw4MgE4CzvLYgQHQScBZHjswADoJOMv/xx3oPOV/Wx4AnTwDYAB0EnCW/wIAAP//iqqTpQAAAAZJREFUAwC4hpuw0siKdAAAAABJRU5ErkJggg==',
  repeatOne: 'data:image/png;base64,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',
  repeatAll: 'data:image/png;base64,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',
  shuffle: 'data:image/png;base64,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',
  speed: 'data:image/png;base64,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',
  mv: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAADu0lEQVR4AeybW6hMURjHxy3lXiR0orwpIqSklHLJg/CAEi9E5BYPHsQLRXkQOR3K5YVyfcCLSEhJeXDnjUIkkWtKrr9/mjPNmBl7728vc2bv7/T/t9a+fGut7zffmb1n2tO54H8mAg7QhK9QcIAO0EjAGO4V6ACNBIzhXoEO0EjAGO4V6ACNBIzhXoEO0EjAGJ7HCjQiKw93gOU8Ym85wNjIygMcYDmP2FtFgN2IHI8P4ef4O/7VZNaatfaDrHscVk40YSWAmmg+05zFS3AL7oKbTVqz1r6UhZ/Dykm50Q0nARzN8DvxEJwVKRflNCp0QgK4kkk0IU2mpJyUW9CkBHB60BkaO/iM0NML4KDQkzRw/MEpzi1WIxlvKz6MVd09tLMrG1lVWrnpAjURSHvxerwIb8NrBJDWVYeAGE3g+AE8GffCurr3p12tg7SuGgRUwVM4dgaPwJW8Wip3cE5HVEPW1JtZF2LdHw+krSoHWBVLQRfWFRxqxT1xTTnAv9EMZ9cGvAmrCmlqywGW2HSiq9sUgVtGvx/+pxzgH0TiMIaubk0W0EaCx3n+bAwQVHn6PmAP/ZlYtyk00STy0c7M7lljSe00noS741jKM0Dd4+mz8mWI6cKRiEWiICZsdvUhgcX4BFafJpnyClDw9H1h32TYSlF5BTgVBAOwWXkFaAZXHMABFkkkbMMBTLigZgtzgMZXzAE6QCMBY7hXoAM0EjCGewU6wEQELhH1BpuV1wo8ArmN+AM2Ka8AP0JNEOfSvsOJlVeAAqbnCa/QmYYf4584tvIMsAjrFp15+Dr+imPJARYKehL3LtTW4fP4M46sMoCRo7J3ov5975DWZnwcv8eR5ABLmFSJD9ncgfUgUSSIDhBaFXrC9i68HX/CdeUAq+N5xe79eBWu+57oACFUQ6q+YxybjV/jqnKAVbG079S94lW25uBHWBcbmnY9dYDtLGp2BO0mR5fja1j/0t9o3+I2ARRl+plUWrn9gM4NvBbvxkfxFtwqgHrDpJ9JvUwxK1XifcYTOP2iax/9LwJ4gU5WdTF0YgIokmm+UqHXHHX8F5zYhoNKAO8xg74byxJEwVNOD8gtqARQV5STzDIL66ei+sloWm++DPnfpDU/YzZ9DFMup+grN5rash4RQI2hifS1jp4NHsoO/ZBET242k7XmYaxdtxu3aZUTTVgVAYadJcOjO0Dji+sAHaCRgDHcK9ABGgkYw70CHaCRgDHcK9ABGgkYw70CcwjQmHK64V6BRp4O0AEaCRjDfwMAAP//boXObgAAAAZJREFUAwAdV5qhV50t9wAAAABJRU5ErkJggg==',
  quality: 'data:image/png;base64,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',
  eq: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAAIx0lEQVR4AeybBYwsxRaGh+funodD8OBOcHd3S3AnBAgkuDsJFiwQ3DUkuAcLLgGCe3B3/76B7q2t23e3e6qHnr3pm/NveXXVPyWnTtX9Waf9l8RAS2ASfZ1OS2BLYCIDicXbEdgSmMhAYvF2BLYEJjKQWLwdgS2BiQwkFm9HYEtgIgOJxfs1An9Bu/4Ppga/Ar3I3yk0Dfgj6EX+QKHpwH/ARCCTWt1+EChh69DKs8DZ4ATwZ1BW7OxiZD4VWMdpuLODKjIDmU8BZwLLr4Trj4pTr/SDwH/QxMPBQmAusBHYEZSVScm4HVgRzAlWAbsBRxTOqCJRu5JrdTAHWBrsAKYCtUs/CJyCVv4bOJJwOnZofj0l4dS1s1nbfk65ScD/QBnxB5yMjH4Xp2M7psRjvTj1StbIOmvNGh7WWRQXpod+Oxy3qyguLBP6LWv+0eLC9J79fqznwm3BTmsPTB0E7QhMZHDQCPwl/XG3jddMw0XxZG9WBoXAiaDBnXcz3P2BqgxOLirUBxFaD5TdjcnafxkEAlVTFqerx4FjwAJAZRwnl9/hWwKY5yjcWcFASNMEOvJUuB1dEiSZIxHjNF6TDBI9E27j0jSBTtXtYWE2ULYtkj4v+fcEfwGNStlG96ORjraFqXg5ELbjW8LvgdvBOeDeTqfzIe53IBPLOnKXJ0JCcZqRsOE/dQucjqvyUXdYnK5I3n341gWui+vjegx0c3kcf0jiPwkvCKoYKsherzRJ4O/pyjwglI8JHAKuBV8C5Sv+XA6OB++CTBx50xP4L2hMmiTQw70H/7DzrxK4BoQjjWBHMp3SLxoIoNHiT0H4J/c2SaBT2FEUdvp9Ap+BIvmAyE9BKL8hoPKN04w0SeDbdNk1DycXzVbxqMwSJ8bzNxDKRwQ+B41JkwQ62p6Nei5BWxIXT0unuwZSbY0k5+KUficPNeBpksBP6O/1IJTfEtgKaIHWrD8z/qXAPsAd2SmLtyuui/fjex00Jk0S6NS7gp6/AkLxrKsJ/kQiTwfeqWyOq9qCk8tz+Nxwxrdmktx/aZJA17876eK5wNGEk4tnX40LnnmdtvHZWPJVsu/OSzTkaZJAu6zep3HgUgJfgDLiiPO2Tn3x6zIF+pmnaQLt2xv88RbuWNynQDwaieqKxD2Cb2+wNWicPNrQGQQCbYcqjRvHFgROBu7QOLm4UUjwJsQ4YgeCPNrSRwKtvRq+IfstwMv4t3BD8YTiJbnnZPOFaY36B2UEhiS4uYgwzqNdHBemh36NE3G/fB6yKJnGp6ST1JvEH+qtlsEopY64DE25CMRGChXzfYlX8T4A1/cytfS9lkpoUNOiYXUnGnE+8DmJ9kK844jqkWutKtDcpDpacXqXCYFATy87Q4GnFUca3hFFcjXGenk1LTljgwZR5WVCINBHSJ6fq1hlJFFjrLu+o7I8Y1HOfhD46+gbBrWkLILHdQqnNtH44Osv3bBSNxyPiV5ALUuCSreqEN5cJNyyPkTKI6t66iLQejzDHkkDLgOxeCzTcHATCUuCuoj0+Zt3xuE09Ji3B99YC7ihXI27O5BM1SG8ubg7e32QR1T12PGqZeL8/pLeW1xAgu8AXZPwjiNOG2/TXOg1FpRZr8apJIrw2VpsZLiRPOeB+GioRVslnKRh4sXWsIgqgVQC/eVn5IMHgvlAmfr+Sj5VCsmOjQQkVRKnrncrYaHHCMQjjaiuFM2ORqewjd+GpjmyypBH1q64Tm6Lz/UJp1Zx/VPxLqq06BTjICjKWyquSqeLKnT98b1KrE85AvajgMZQ16Kr8Med8lSwMfGuQzg9ifckGhnCwtnD8jAu8xf9YLY1S6/sphK4IV+MNwS1fd81azW5jvQLwRrAaRtaWvzl7WzVB+RUlcsz+N4EobhJ+ajcNdZvmOZSMQseNxOcYXLDsFDFwDACK5Y1uwqpboiDCdwLQnFn1EhwaxiJ/1/A/wqB05M8SKkngdMWpyvqdaotQh3R453mMg23qlPdTD/+0epzyY/+npxUAosWYPWvosY4VR6NEuysm0oUXTqobuexTMNsWMgrU+9WVGMuJuEw4GjPRiTBjuuhGsETBnpFKoFF37VhRfHGxeugcanwB/P+pKgeVSx/pLiftsPrhJMoFN81E1Ve4orLl/whp6PqB9/QX6fMUGjIpwXEUTAU0+m4AbgRhHFV/Y4+19dDKRhOZYKFInkPkeJ66IwwTLA3SSXw5oLPunmoG/rrm+y08QGQD4m8qjQugxuAZvws3KvrKPK5m7u6a2KRxVrF2rc1roUr86E7wEizheTRJZVANX5fB4RfmpyAa4t6nqPRxh5BnHY49T+8XXG0+OLKe45uROIfHyF52bQ29TiycHLxDtr/OuZTOu+XX8pTEj2pBD7M971Rkwy8XXHE+X/VJM0FXDVmU1LiI5474BnE1/2ywPokjKpzcZnwqFn7NWgqgTbUZ2f35E0d8li3C3isZJvDdcdLIi/GDY9Z2MmUxkvEA1SglTeeNkQXiutTtui7iRRmGiuRqQTaTxfi2/CsAJyuPsfFO0yc4u6Wrnce/XxUPubJs4d1EGg9wiOcC7h3E4ZDvExAo4ObiiS74BM19qVOAmXDKf2CngjPE/Ze9zXcCUrqJnDMkZPa4JbARAZbAlsCExlILN6PEajKEjfLzSWOGykc5zcsRiqTpRXlM05keWpz+0GgNjqV5bCRqjhheCS/+qFHrzCPcR79wrjx+S2rzhmm+1/HNDiEcbX4+0Ggb549d2YNVLE+OguUcH37rJld64nZJe9KPFpucEYVj5caMzIS1Tm9Wnh61JI9ZOgHgVpn/B+YG9CeXYB3EZ5A8JYSR4rXpFpNtPGtRinvc4uWBpLGEaeqRgrvRnwv48X5XuSyXTj1Sj8ItIXa3bwD0SKjEm1cFTjqtOR4xvZlgaRUKW/eu/jjmdt6HJUE65d+EVh/Swe0xpbAxB+mJbAlMJGBxOJjcQQmdrne4i2BiXy2BLYEJjKQWPx7AAAA//85+dWkAAAABklEQVQDAL11SbDhmhBSAAAAAElFTkSuQmCC',
  surround3d: 'data:image/png;base64,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',
  record: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAFAAAABQCAYAAACOEfKtAAAFAElEQVR4AeybWagURxSG52YjK9lDFhISkpA8hCQkIYEshBCSkI0kBLIouCsqiqIo7oqKvrgg7qiIT+IuvuiTiKAiKoL4oCIqiPrghrsoLt/feseedqbv1Jw7985Mn8v5p7q661RXf326u7pu1wM5/zMRcIAmfLmcA3SARgJGd49AB2gkYHT3CHSARgJGd49AB2gkYHSvpQhs4lheQF+h39GfgfqD8j+iD9ETKM2+YGNa/b+x/TP0CEq1WgEoeK/R0v5oMVqN1gRKPkvxmYX6oFeR6iW5z4axJq3+VWyfh35GqVYrAF+ilYNQP/QuegiFmo7lOZy+QSPRUPQYSlo5eUXepxQch0qdBDblauJV7klaMgp1RQJAYrZnqKEb+gVZ7D2cm1BJ01krubENNjzMPqYjwdNBs9hq9hQ19UQW24DzTVTS2hOg4C2kZV1QSzd9ilRkelhU4ngJp+VItxSS0tZeAHXZzqRZ/6Pk/U5nfCvr30e6fMrVX5RP2tPJFXfzKptWr9r3L2WPo1RrD4B6YEykVR2RopAkb9dY2owGoAOo5q0YQJ2ZV2j5l+hXlNZfCt2mM68nWyfq1VkmKbBz5Hag15H6dSH1f45Pm1sSoCJC4EbTkkVoJUrrL4VuU19NfbRnqbeYvcjKIUjlQusejl+bWxygIk9ncTyt6I50D3qU1C2FQBzgy5TTfelrUnUkSdxaIhAH+BaFv0MODwjlWhyg7ktvluvo5e4QiAPUA8TveXe4NP+Kid7NNUhxkpWb0E8o33eNA2R9rVqbt+tB9qjRod6k29B/SENt35KORR+gyEIAnsBjbQXCpa5Mt7IfaPEcNA09j+L2MRnBJckFjcZsx0Md4VDhVhemy1WDsQNprcYUNaiav1RZ12waIlPZKB8SgZFDg/7onbkDxzYVaVzybdKy2JRViMoa2T7i4GYgvUB8T1rsFZPVxS3LAB8HiQYt9LqqUaE3yDehIMsqwE+gpAfiZNJ3UMUvD1kFqMESPWn1QIBf5ZZVgJUTS3iGANQYYcj4XHPZxC4bKxsCUP9fCB2jU/nGIpY4mhCACVfPioADFAWDqgfQ0Kh6cnWAxrPlAB2gkYDR3SPQARoJGN09Ah2gkYDR3SPQARoJGN09Ah2gkYDR3SOwNQEa68qku0eg8bRnFaD+lbkfdreQybIKUP9q0Ff4S6CnWQEklVlWAV4G1x7UF/2NjqCKLKsABUuX7xUW9A92fVg/l+ULKCgiswwQVnnTp3uDyenjek3y0Uwlsi2bA7zHSNG4gqzm180nPYiKReN11t9AkTnACEPBzz5y+vRD02XXsaz7JUneDrN0BkXmACMM9/0Imp7UuqzHsHUnuog0/UwTJNUFIpsr+EJVYanwjDY0yI8uS8uhHMJ5NtK30v+Qag6yZtSfZjmyeASKsL5EjzY0yI/uY9ZDuUoFu9B6tAWdQnqCkxRG4FHWKFRJGsaWtXQk1u3xCFS4aofHqDRPmOV6ND09d9Nw9e1IqmdxgNqpOpWT2N1edB7pvkhSF6aTrvnGusQ20uIe6CyqqsUBakd6+qgPpA7lFFYoIgW1HqSnpt5tR9DuzkgRKKgsVs+SALUnRZ0mPU8go9mbofNC2qu83ml70eYFSFP1qw6P/RR0Y5R3BRIoFoGBVWS7uAM0nn8H6ACNBIzuHoEO0EjA6O4RmEGAxkNuXXePQCNPB+gAjQSM7rcBAAD//8hpf0kAAAAGSURBVAMAlAMCsH9RcsoAAAAASUVORK5CYII=',
  download: 'data:image/png;base64,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',
  share: 'data:image/png;base64,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',
  add: 'data:image/png;base64,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',
} as const

export type PlayerIconKey = keyof typeof PLAYER_ICONS
```

**着色（`GoIcon.vue` 片段）**

```vue
<template>
  <span class="go-icon" :style="{ '--icon': `url(${PLAYER_ICONS[name]})` }" />
</template>

<style scoped>
.go-icon {
  display: inline-block;
  width: 24px; height: 24px;
  background: currentColor;
  color: var(--go-text);
  -webkit-mask: var(--icon) center / contain no-repeat;
  mask: var(--icon) center / contain no-repeat;
}
.go-icon.is-active { color: var(--go-accent); }
.go-icon.is-disabled { opacity: .35; }
</style>
```

**校验**：每枚源图须为 `80×80 / RGBA`；base64 解码后与包内文件字节一致；§6.1.4 表中 17 个 key 与本节常量一一对应，缺一即构建期报错。图标位图只作形状蒙版，任何主题色变化都通过 `currentColor` 完成，不生成第二套资源。
