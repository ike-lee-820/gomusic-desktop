/**
 * 自动编排版本号：yy.mm.次数（如 26.11.1）。
 * - 「次数」= 当前年月下已有 v<yy>.<mm>.<n> tag 的最大 n + 1，换月自动回到 1。
 * - build 单调递增、不回退，写入 version.json。
 * - 每次改写全部版本文件，并生成只含「新增 / 修复」两栏的更新文档，
 *   归档 md、内置 CHANGELOG.md、Release 正文三者逐字一致。
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { execSync } from 'node:child_process'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const repo = process.env.GITHUB_REPOSITORY || 'ike-lee-820/gomusic-desktop'
const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN || ''

async function listTags() {
  if (!token) return []
  try {
    const res = await fetch(`https://api.github.com/repos/${repo}/tags?per_page=100`, {
      headers: {
        'User-Agent': 'GoMusicReleaseBot',
        Accept: 'application/vnd.github+json',
        Authorization: `Bearer ${token}`,
      },
    })
    if (!res.ok) return []
    const data = await res.json()
    return Array.isArray(data) ? data : []
  } catch {
    return []
  }
}

const tags = await listTags()
const now = new Date()
const parts = Object.fromEntries(
  new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit',
  }).formatToParts(now).map((p) => [p.type, p.value]),
)
const prefix = `${parts.year.slice(2)}.${parts.month}`

let maxN = 0
let maxBuild = 0
let lastTag = ''
let samePrefixTag = ''
for (const t of tags) {
  const m = /^v(\d{2})\.(\d{2})\.(\d+)$/.exec(t.name || '')
  if (!m) continue
  const n = Number(m[3])
  const isSameMonth = `${m[1]}.${m[2]}` === prefix
  if (isSameMonth) {
    if (n > maxN) { maxN = n; samePrefixTag = t.name }
  }
  if (n >= maxBuild) { maxBuild = n; lastTag = t.name }
}
// 换月后 range 从上月最后一个 tag 起算，避免把基线历史塞进本次说明。
if (samePrefixTag) lastTag = samePrefixTag

let next = maxN + 1
let current = ''
try { current = JSON.parse(fs.readFileSync(path.join(root, 'version.json'), 'utf8')).version } catch { /* 首次 */ }
// 每次发布都必须更改版本号：即使 tag 读取失败也不沿用当前版本。
if (`${prefix}.${next}` === current) next += 1
const version = `${prefix}.${next}`
const tag = `v${version}`

let prevBuild = 0
try {
  prevBuild = Number(JSON.parse(fs.readFileSync(path.join(root, 'version.json'), 'utf8')).build) || 0
} catch { /* 首次运行 */ }
const build = Math.max(prevBuild + 1, maxBuild + 1)

const override = (process.env.RELEASE_NOTES || '').trim()

function subjectNotes() {
  let subjects = []
  try {
    const range = lastTag ? `${lastTag}..HEAD` : 'HEAD'
    subjects = execSync(`git log ${range} --pretty=format:%s -n 100`, { cwd: root, encoding: 'utf8' })
      .split('\n').map((s) => s.trim()).filter(Boolean)
  } catch {
    try {
      subjects = execSync('git log --pretty=format:%s -n 100', { cwd: root, encoding: 'utf8' })
        .split('\n').map((s) => s.trim()).filter(Boolean)
    } catch { /* 无 git 历史 */ }
  }

  const added = []
  const fixed = []
  for (const s of subjects) {
    // 发布编排提交不是功能变更，不进入对外文案。
    if (/\[skip ci\]|^chore\(release\)/i.test(s)) continue
    let text = s.replace(/^(feat|fix|perf|refactor|docs|chore)(\([^)]*\))?!?:\s*/i, '').trim()
    text = text.replace(/\s+/g, ' ').slice(0, 48)
    if (/^(feat|新增|add|支持)|\b新增\b|\b实现\b/i.test(s)) added.push(text)
    else if (/^(fix|修复|修正|hotfix)|\b修复\b|\b修正\b/i.test(s)) fixed.push(text)
  }
  const uniq = (a) => [...new Set(a.filter(Boolean))].slice(0, 6)
  const A = uniq(added)
  const F = uniq(fixed)
  // 只有「新增 / 修复」两栏，无版本标题、无排错/核验/构建过程。
  const lines = ['## 新增', '']
  for (const x of (A.length ? A : ['无'])) lines.push(`- ${x}`)
  lines.push('', '## 修复', '')
  for (const x of (F.length ? F : ['无'])) lines.push(`- ${x}`)
  lines.push('')
  return lines.join('\n')
}

/** 只保留「新增 / 修复」两栏；自定义输入也做一次收敛，避免出现排错/构建过程。 */
function onlyNewFix(md) {
  const keep = []
  let section = ''
  for (const line of md.split('\n')) {
    const h = /^#{1,6}\s*(.+?)\s*$/.exec(line)
    if (h) {
      const t = h[1].replace(/[:：].*$/, '').trim()
      section = /^(新增|修复)$/.test(t) ? t : ''
      if (section) keep.push(`## ${section}`)
      continue
    }
    if (section) keep.push(line)
  }
  return keep.join('\n').trim() + '\n'
}

const notes = onlyNewFix(override || subjectNotes())

const w = (rel, content) => {
  const p = path.join(root, rel)
  fs.mkdirSync(path.dirname(p), { recursive: true })
  fs.writeFileSync(p, content)
}

// 三处同源，逐字一致
w(`docs/release-notes/${version}.md`, notes)
w('src-tauri/resources/CHANGELOG.md', notes)
w('version.json', JSON.stringify({ version, build, tag, updatedAt: now.toISOString() }, null, 2) + '\n')

const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))
pkg.version = version
w('package.json', JSON.stringify(pkg, null, 2) + '\n')

const conf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'))
conf.version = version
w('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n')

let cargo = fs.readFileSync(path.join(root, 'src-tauri/Cargo.toml'), 'utf8')
cargo = cargo.replace(/^version = ".*"$/m, `version = "${version}"`)
w('src-tauri/Cargo.toml', cargo)

w('src/version.ts',
  '// 由 scripts/bump-version.mjs 自动生成，请勿手工修改。\n' +
  '// 版本号规则：yy.mm.次数（如 26.11.1），每次发布自动递增，构建号单调递增不回退。\n' +
  `export const APP_VERSION = '${version}'\n` +
  `export const BUILD_NUMBER = ${build}\n` +
  `export const BUILD_TAG = '${tag}'\n`)

const out = process.env.GITHUB_OUTPUT
if (out) {
  fs.appendFileSync(out, `version=${version}\ntag=${tag}\nbuild=${build}\nnotes_path=docs/release-notes/${version}.md\n`)
}
console.log(`version=${version} tag=${tag} build=${build} prefix=${prefix} tags=${tags.length}`)
console.log('--- release notes ---')
process.stdout.write(notes)
