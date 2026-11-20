# GoMusic Desktop

Rust + Tauri 2 + Vue 3 桌面客户端，对应 `docs/GoMusic-电脑端开发文档-Rust-Tauri2-Vue3.md`。

**核心约束**：仅横屏窗口（默认 1280×800，最小 960×600），不渲染竖屏分支、不设竖屏断点；左导航恒 76px。

## 版本号与发布（全自动）

每次 push 到 `main`（或手动触发 workflow_dispatch）都会自动跑一遍：

1. **编排版本号**：`yy.mm.次数`，例如 `26.11.1`、`26.11.2`……「次数」取当前年月下已有 `v<yy>.<mm>.<n>` tag 的最大值 +1，换月自动回到 1。`build` 单调递增、绝不回退，记录在 `version.json`。
2. **改写全部版本文件**：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src/version.ts`、`version.json`；版本号在导航条、关于页、Rust User-Agent 里同步生效。
3. **生成更新文档**：只含「新增」「修复」两栏，简洁明了。`docs/release-notes/<版本>.md`、`src-tauri/resources/CHANGELOG.md`、GitHub Release 正文**三者逐字一致**，不含排错、核验、构建过程。
4. **打 tag + 建 Release**：`v<版本>`，正文即归档 md。
5. **真产物**：五个平台 `tauri build` 产出安装包（Windows x64/ARM64、macOS arm64、Linux amd64/arm64），上传到 GitHub Release 的 Assets，同时作为 Actions Artifact 留存，并附按平台命名的 `SHA256SUMS-<平台>.txt`。产物包括 `.exe` / `.msi` / `.dmg` / `.deb` / `.rpm` / `.AppImage`。

手动触发时可填 `notes` 自定义更新说明（仍会被收敛为只保留「新增 / 修复」两栏）。版本号由 `scripts/bump-version.mjs` 统一编排，请勿手工改版本文件。

## 启动自动检查更新

应用启动即调用 `update_check`：

- **默认加速**：`https://gh-proxy.com/https://api.github.com/...`，同时并发取 `v6.gh-proxy.com` 与 `api.github.com` 直连。
- **403 自动回退**：gh-proxy 被 Cloudflare WAF 拦截、返回 HTML 403 页、超时或响应不是 JSON 时，该候选直接丢弃，自动用直连结果，界面不会出现英文堆栈。
- **取最完整候选**：按首个 Release 的 `body` 长度比较，短于最长 90% 或不像 Markdown 的不采用。
- **有新版本**：底部弹出提示 → 「查看更新说明」进 `/update-doc`，「下载更新（加速）」走 `gh-proxy.com/github.com` 前缀，另给「直连」按钮兜底。
- **全部不可用**：读内置 `resources/CHANGELOG.md`，离线完整可读；检查失败不阻断启动。

更新说明页用 `markdown-it` + DOMPurify 渲染，底部无字数统计、无「复制全文」、无「查看完整文档」行。

## 快速开始

```bash
npm install
npm run tauri dev      # 开发
npm run tauri build    # 本地打包
npm run typecheck      # 类型检查
```

Rust 侧检查：`cargo check --manifest-path src-tauri/Cargo.toml`（需先存在 `dist/index.html`）。

## 目录

```
scripts/bump-version.mjs          版本号编排 + 更新文档生成
version.json                      当前版本 / 构建号
docs/release-notes/<版本>.md      归档更新说明（Release 正文同源）
src/version.ts                    前端版本常量（自动生成）
src/                              Vue 3 前端（views / components / canvas / api / stores）
src-tauri/resources/CHANGELOG.md  内置离线更新说明
src-tauri/                        Rust 宿主：commands / net / audio / dsp / local / db / rec / media / sys
.github/workflows/build.yml       push/PR 校验（前端构建、cargo check、凭据扫描、版本一致性）
.github/workflows/release.yml     自动版本 + 六平台产物 + GitHub Release
```

## CI

- `build.yml`：前端 `vite build`、Rust `cargo check`、配置/凭据/参考资源校验、版本文件一致性检查。
- `release.yml`：push 到 `main` 触发，先编排版本并建 Release，再按矩阵构建并上传安装包。ARM runner 标记为实验性，失败不影响整体发布。

## 与文档的差异（诚实清单）

本仓库是**可编译、CI 全绿、能出安装包的工程基线**，不是功能完整的客户端：

- `net/` 已实现通用请求封装、音质降档链（风控码立即终止）、`/lyric` 免登录快速通道、字段容错、更新检查（gh-proxy 优先 + 403 回退 + 最长候选）。
- `commands/` 覆盖 §2.1 全部 IPC 命令签名；音频解码、DSP、录制编码、媒体键、本地元数据等仍为**桩接口**（`audio/ dsp/ rec/ media/ local/ db/ sys/`），待接入 `symphonia / cpal / lofty / souvlaki / rustfft / ffmpeg-next`。
- 雨雪特效：`reference/` 下已放入雨特效上游原文件（`index.html / style.css / script.js` 及 `rain26.js` 逐字副本）；`sn.html` 上游返回 403 暂缺，缺失时 CI 只告警不阻断。
- `bundle.externalBin` 与 `plugins.updater` 因缺少 ffmpeg sidecar / minisign 密钥未启用；本版「检查更新」是提示 + 下载链接，不是带签名的静默安装。补齐密钥后可接入 `tauri-plugin-updater`。
- macOS Developer ID 签名 + notarize、Windows Authenticode 证书待补；未签名包可能被 Gatekeeper / SmartScreen 提示。
- 播放器 17 枚图标由文档 §14 的 `src/assets/icons.ts` 原样内嵌，零外部依赖；安装图标由同一份 PNG 生成 `.ico` / `.icns`。

## 安全

- 前端 bundle 视为不可信：凭据、cookie 只留在 Rust 侧，不进 JS。
- 任何 token / 私钥不得写入仓库；CI 有凭据泄漏扫描，Release 只用 `GITHUB_TOKEN`。
- capabilities 为最小权限集，默认拒绝；CSP 显式配置并放行封面直链与更新加速域。

## 许可证

与手机端仓库保持一致，待维护者确认。
