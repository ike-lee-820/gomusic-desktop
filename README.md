# GoMusic Desktop

Rust + Tauri 2 + Vue 3 桌面客户端工程基线，对应 `docs/GoMusic-电脑端开发文档-Rust-Tauri2-Vue3.md`（手机端 v26.11.36 / versionCode 138）。

**核心约束**：仅横屏窗口（默认 1280×800，最小 960×600），不渲染竖屏分支、不设竖屏断点；左导航恒 76px；与手机端横屏布局对齐。

## 快速开始

```bash
npm install
npm run tauri dev      # 开发
npm run tauri build    # 打包（需平台工具链）
```

前端单独预览：`npm run dev` → http://127.0.0.1:5173
类型检查：`npm run typecheck`

Rust 侧检查：`cargo check --manifest-path src-tauri/Cargo.toml`（需先存在 `dist/index.html`，CI 已处理）。

## 目录

```
src/                     Vue 3 前端（views / components / canvas / api / stores）
src-tauri/               Rust 宿主：commands / net / audio / dsp / local / db / rec / media / sys
src-tauri/resources/     内置 CHANGELOG 与参考特效（只读）
docs/                    完整开发文档
.github/workflows/       CI 构建与发布矩阵
```

## CI

- `build.yml`：push / PR / 手动触发。前端 `vite build`、Rust `cargo check`、配置与凭据泄漏扫描、参考资源校验和校验。
- `release.yml`：`v*` tag 或手动触发，按 §10 的矩阵产出 Windows x64/ARM64、macOS x64/ARM64、Linux amd64/arm64 安装包，并用 `tauri-apps/tauri-action` 发布到 Releases。需要 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 等 Secrets。

发布前需补齐：
1. `npm run tauri icon src-tauri/icons/icon.png` 生成全平台图标集（Windows .ico / macOS .icns）。
2. `tauri signer generate` 生成 minisign 密钥对，公钥写入 `tauri.conf.json` 的 `plugins.updater.pubkey`，私钥写入 CI Secrets。
3. 各 target triple 的 ffmpeg sidecar 放入 `src-tauri/binaries/`，并恢复 `bundle.externalBin`。
4. macOS Developer ID 签名 + notarize、Windows Authenticode 证书。
5. `resources/reference/sn.html`（上游当前返回 403，需由维护者提供原文件）；放入后**逐字不改**，并更新 `reference/SHA256SUMS`。
6. 可变字库 `SourceHanSans-VF.otf` / `SourceHanSerif-VF.otf`。

## 与文档的差异（诚实清单）

本仓库是**可编译、CI 可构建通过的工程基线**，不是功能完整的客户端：

- `net/` 已实现通用请求封装、音质降档链（含风控终止码）、`/lyric` 免登录快速通道、字段容错解析。
- `commands/` 覆盖 §2.1 全部 IPC 命令签名与返回结构；音频解码、DSP、录制编码、媒体键、本地元数据等为**桩接口**（`audio/ dsp/ rec/ media/ local/ db/ sys/`），接口与数据模型已按 §5 冻结，待接入 `symphonia / cpal / lofty / souvlaki / rustfft / ffmpeg-next`。
- 雨雪特效：`reference/` 下已放入雨特效上游原文件（`index.html / style.css / script.js`，并保留 `rain26.js` 逐字副本）；`sn.html` 因上游 403 暂缺，缺失时 CI 只告警不阻断。
- `bundle.externalBin` 与 `plugins.updater` 因缺少 sidecar / 签名密钥暂未启用；`fileAssociations` 中 macOS `exportedType` 待补；`bundle.icon` 目前用单张 `icon.png`，发布时生成全套。
- 播放器 17 枚图标由文档 §14 的 `src/assets/icons.ts` 原样内嵌，零外部依赖。

## 安全

- 前端 bundle 视为不可信：所有凭据、cookie 只留在 Rust 侧（`reqwest::cookie::Jar` / `keyring` 待接入），不进 JS。
- 任何 token / 私钥不得写入仓库；CI 有 `ghp_` / `github_pat_` 泄漏扫描。
- capabilities 为最小权限集，默认拒绝；CSP 显式配置并放行封面直链域。

## 许可证

与手机端仓库保持一致，待维护者确认。
