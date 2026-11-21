/**
 * 汇总各平台 CI 产出的 updater 包及其 minisign 签名，生成 Tauri 2 updater 的 latest.json。
 * 未签名、签名与公钥不配对、或对应包缺失的平台一律不写入，保证「未签名更新会被拒绝」。
 */
import fs from 'node:fs'
import path from 'node:path'

const tag = process.env.TAG
const version = process.env.VERSION
const repo = process.env.GITHUB_REPOSITORY || 'ike-lee-820/gomusic-desktop'
const root = process.cwd()
const base = path.join(root, 'all-artifacts')

const notice = (m) => console.log(`::notice::${m}`)
const warn = (m) => console.log(`::warning::${m}`)

let notes = ''
try { notes = fs.readFileSync(path.join(root, 'docs/release-notes', `${version}.md`), 'utf8').trim() } catch { /* 离线兜底 */ }
if (!notes) notes = '## 新增\n\n- 无\n\n## 修复\n\n- 无'

/** minisign 公钥：base64 -> "Ed" + keynum(8) + pk(32)。 */
function pubkeyKeynum(pubkey) {
  const b = Buffer.from(pubkey, 'base64')
  if (b.length < 10) return ''
  return b.subarray(2, 10).toString('hex')
}
/** Tauri 生成的 .sig 是「外层 base64 包装的 minisign 文本」；解开后签名行为 "ED" + keynum(8) + sig(64)。 */
function sigKeynum(text) {
  let body = text.trim()
  // 外层 base64：解开后应得到含 untrusted/trusted comment 的文本
  try {
    const outer = Buffer.from(body, 'base64').toString('utf8')
    if (outer.includes('untrusted comment') || outer.includes('trusted comment')) body = outer
  } catch { /* 已是明文 minisign 文本 */ }
  const line = body.split('\n')
    .map((l) => l.trim())
    .find((l) => l && !l.startsWith('untrusted comment') && !l.startsWith('trusted comment'))
  if (!line) return ''
  const b = Buffer.from(line, 'base64')
  if (b.length < 10) return ''
  return b.subarray(2, 10).toString('hex')
}

let expectedKeynum = ''
try {
  const conf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'))
  expectedKeynum = pubkeyKeynum(conf.plugins.updater.pubkey)
} catch { /* 配置不可读时仅跳过配对校验 */ }
notice(`updater 公钥 keynum=${expectedKeynum || '(未知)'}`)

const priority = ['.app.tar.gz', '.nsis.zip', '.msi.zip', '.AppImage.tar.gz', '.AppImage', '.exe', '.msi', '.deb', '.rpm']
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
const seen = []
if (fs.existsSync(base)) {
  for (const dir of fs.readdirSync(base)) {
    const p = path.join(base, dir)
    if (!fs.statSync(p).isDirectory()) continue
    const key = platformKey(dir)
    const files = fs.readdirSync(p)
    const sigs = files.filter((f) => f.endsWith('.sig')).sort((a, b) => score(a) - score(b))
    seen.push(`${dir}: sig=[${sigs.join(',')}] key=${key || '-'}`)
    if (!key || platforms[key] || !sigs.length) continue
    const chosen = sigs[0]
    const signature = fs.readFileSync(path.join(p, chosen), 'utf8').trim()
    if (!signature) continue
    const asset = chosen.slice(0, -4)
    // 对应的 updater 包必须一并上传，否则客户端下载 404。
    if (!files.includes(asset)) { warn(`${key}: 缺少包 ${asset}，跳过`); continue }
    const kn = sigKeynum(signature)
    if (expectedKeynum && kn && kn !== expectedKeynum) {
      warn(`${key}: 签名 keynum=${kn} 与公钥 ${expectedKeynum} 不配对，跳过`)
      continue
    }
    notice(`${key}: ${asset} keynum=${kn || '?'}`)
    platforms[key] = {
      signature,
      url: `https://github.com/${repo}/releases/download/${tag}/${asset}`,
    }
  }
}
for (const s of seen) notice(`artifact ${s}`)

const out = { version, notes, pub_date: new Date().toISOString(), platforms }
fs.writeFileSync(path.join(root, 'latest.json'), JSON.stringify(out, null, 2) + '\n')
const keys = Object.keys(platforms)
notice(`latest.json 平台数=${keys.length}: ${keys.join(', ') || '(空)'}`)
if (!keys.length) warn('没有任何已签名平台，签名安装将不可用；请检查 TAURI_SIGNING_PRIVATE_KEY 与公钥是否配对')
