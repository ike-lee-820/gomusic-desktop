/**
 * 汇总各平台 CI 产出的 updater 包及其 minisign 签名，生成 Tauri 2 updater 的 latest.json。
 * 未签名的平台不写入清单，保证「未签名更新会被拒绝」。
 */
import fs from 'node:fs'
import path from 'node:path'

const tag = process.env.TAG
const version = process.env.VERSION
const repo = process.env.GITHUB_REPOSITORY || 'ike-lee-820/gomusic-desktop'
const root = process.cwd()
const base = path.join(root, 'all-artifacts')

let notes = ''
try { notes = fs.readFileSync(path.join(root, 'docs/release-notes', `${version}.md`), 'utf8').trim() } catch { /* 离线兜底 */ }
if (!notes) notes = '## 新增\n\n- 无\n\n## 修复\n\n- 无'

const priority = ['.nsis.zip', '.msi.zip', '.app.tar.gz', '.AppImage.tar.gz', '.AppImage']
const score = (f) => {
  const i = priority.findIndex((x) => f.endsWith(`${x}.sig`))
  return i === -1 ? 99 : i
}

function platformKey(dirName) {
  const d = dirName.toLowerCase()
  const arm = /arm64|aarch64/.test(d)
  if (d.includes('windows')) return arm ? 'windows-aarch64' : 'windows-x86_64'
  if (d.includes('macos') || d.includes('darwin')) return arm ? 'darwin-aarch64' : 'darwin-x86_64'
  if (d.includes('ubuntu') || d.includes('linux')) return arm ? 'linux-aarch64' : 'linux-x86_64'
  return null
}

const platforms = {}
if (fs.existsSync(base)) {
  for (const dir of fs.readdirSync(base)) {
    const p = path.join(base, dir)
    if (!fs.statSync(p).isDirectory()) continue
    const key = platformKey(dir)
    if (!key || platforms[key]) continue
    const sigs = fs.readdirSync(p).filter((f) => f.endsWith('.sig')).sort((a, b) => score(a) - score(b))
    const chosen = sigs[0]
    if (!chosen) continue
    const signature = fs.readFileSync(path.join(p, chosen), 'utf8').trim()
    if (!signature) continue
    const asset = chosen.slice(0, -4)
    platforms[key] = {
      signature,
      url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
    }
  }
}

const out = { version, notes, pub_date: new Date().toISOString(), platforms }
fs.writeFileSync(path.join(root, 'latest.json'), JSON.stringify(out, null, 2) + '\n')
console.log('latest.json platforms:', Object.keys(platforms).join(', ') || '(none)')
