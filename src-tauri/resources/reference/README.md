# 参考实现（只读）

本目录文件**逐字不改**，按 §8 原样在 WebView 中运行；任何格式化或改写都会破坏「严格不变」约束。CI 会用 `SHA256SUMS` 比对，上游更新时整文件替换并重新生成校验和。

| 文件 | 来源 | 用途 |
|---|---|---|
| `index.html` / `style.css` / `script.js` | `Lavender-z/demo` 的「雨特效（css+js）」目录 | 雨季 WebGL2 参考实现，原文件 |
| `rain26.js` | `script.js` 的逐字副本 | 按文档 §2.3 命名，内容一字不改 |
| `sn.html` | `https://cloud.myocd.de5.net/go/sn.html` | 飘雪参考实现 |

`sn.html` 上游当前返回 403，暂未纳入仓库；缺失时 CI 只告警不阻断。请由维护者提供原文件后放入本目录，并更新 `SHA256SUMS`。**禁止用「等价实现」替代原文件。**

`../fonts/` 下的 `SourceHanSans-VF.otf` / `SourceHanSerif-VF.otf` 同样待提供，前端 `@font-face` 已按可变字重 `100–900` 预留。
