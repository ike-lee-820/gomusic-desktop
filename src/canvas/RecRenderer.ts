import { computeArcFrame, drawArcFrame } from './ArcLyrics'
import type { RecConfig, Song } from '../api/types'

/** 录制预览与编码物理同源：页面 canvas 就是本文件输出，逐帧像素直接送 Rust。 */

export interface FrameOptions {
  width: number
  height: number
  song: Song | null
  lines: string[]
  focus: number
  config: RecConfig
  positionMs: number
  durationMs: number
  accent: string
  cover: HTMLImageElement | null
}

let lastSignature = ''
let reused = 0
let rendered = 0

/** 画面签名：高亮行、进度条每 2px、样式/背景/字号/字体/偏移，相同则复用上一帧。 */
export function frameSignature(o: FrameOptions): string {
  return [
    o.width, o.height, o.song?.id, o.focus,
    Math.floor(o.positionMs / 2000),
    Math.floor((o.durationMs ? o.positionMs / o.durationMs : 0) * 500) * 2,
    o.config.style, o.config.background, o.config.lyricSize, o.config.infoSize,
    o.config.fontSource, o.config.wght, o.config.lyricOffsetMs, o.config.showTrans,
  ].join('|')
}

function fitCover(ctx: CanvasRenderingContext2D, img: HTMLImageElement, w: number, h: number) {
  const scale = Math.max(w / img.width, h / img.height)
  const dw = img.width * scale
  const dh = img.height * scale
  ctx.drawImage(img, (w - dw) / 2, (h - dh) / 2, dw, dh)
}

/** 单张封面 cover 铺满 + 轻模糊；不叠第二张、不拉伸、不留纯色边。 */
export function paintBackground(canvas: HTMLCanvasElement, url: string | null): void {
  const dpr = window.devicePixelRatio || 1
  const w = canvas.clientWidth || 800
  const h = canvas.clientHeight || 600
  canvas.width = Math.floor(w * dpr)
  canvas.height = Math.floor(h * dpr)
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.scale(dpr, dpr)
  ctx.fillStyle = '#0B0D10'
  ctx.fillRect(0, 0, w, h)
  if (!url) return
  const img = new Image()
  img.crossOrigin = 'anonymous'
  img.onload = () => {
    ctx.save()
    ctx.filter = 'blur(18px)'
    fitCover(ctx, img, w, h)
    ctx.restore()
  }
  img.src = url
}

export function renderArcFrame(
  canvas: HTMLCanvasElement,
  o: { lines: string[]; focus: number; width: number; height: number; accent: string },
): void {
  const dpr = window.devicePixelRatio || 1
  canvas.width = Math.floor(o.width * dpr)
  canvas.height = Math.floor(o.height * dpr)
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.scale(dpr, dpr)
  drawArcFrame(ctx, computeArcFrame(o.lines, o.focus, o.width, o.height), o.width, o.accent)
}

/** 唯一帧渲染入口；返回 ImageData 供预览与 rec_push_frame 共用。 */
export function renderFrame(canvas: HTMLCanvasElement, o: FrameOptions): ImageData {
  const sig = frameSignature(o)
  const ctx = canvas.getContext('2d', { willReadFrequently: true })
  if (!ctx) throw new Error('Canvas2D unavailable')
  if (sig === lastSignature) {
    reused++
    return ctx.getImageData(0, 0, canvas.width, canvas.height)
  }
  lastSignature = sig
  rendered++

  canvas.width = o.width
  canvas.height = o.height
  if (o.cover) {
    ctx.save()
    ctx.filter = 'blur(10px)'
    fitCover(ctx, o.cover, o.width, o.height)
    ctx.restore()
  } else {
    ctx.fillStyle = '#0B0D10'
    ctx.fillRect(0, 0, o.width, o.height)
  }
  const grad = ctx.createLinearGradient(0, 0, 0, o.height * 0.6)
  grad.addColorStop(0, 'rgba(0,0,0,.45)')
  grad.addColorStop(1, 'rgba(0,0,0,0)')
  ctx.fillStyle = grad
  ctx.fillRect(0, 0, o.width, o.height * 0.6)

  const infoW = Math.round(o.width * o.config.infoZone)
  ctx.fillStyle = '#F2F4F8'
  ctx.font = `600 ${o.config.infoSize + 6}px sans-serif`
  ctx.fillText(o.song?.name ?? '', 40, o.height * 0.5, infoW - 80)
  ctx.fillStyle = 'rgba(242,244,248,.62)'
  ctx.font = `${o.config.infoSize}px sans-serif`
  ctx.fillText(o.song?.artist ?? '', 40, o.height * 0.5 + 28, infoW - 80)

  if (o.config.style === 'DISC') {
    drawArcFrame(ctx, computeArcFrame(o.lines, o.focus, o.width - infoW, o.height), o.width - infoW, o.accent, o.config.lyricSize)
  } else {
    ctx.save()
    ctx.translate(infoW, 0)
    ctx.font = `${o.config.lyricSize}px serif`
    for (let i = -3; i <= 3; i++) {
      const idx = o.focus + i
      if (idx < 0 || idx >= o.lines.length) continue
      ctx.globalAlpha = i === 0 ? 1 : 0.28
      ctx.fillStyle = i === 0 ? o.accent : '#F2F4F8'
      ctx.fillText(o.lines[idx], 40, o.height * 0.5 + i * o.config.lyricSize * 1.6, o.width - infoW - 80)
    }
    ctx.restore()
  }

  const p = o.durationMs ? Math.min(1, o.positionMs / o.durationMs) : 0
  ctx.fillStyle = 'rgba(255,255,255,.18)'
  ctx.fillRect(0, o.height - 4, o.width, 4)
  ctx.fillStyle = o.accent
  ctx.fillRect(0, o.height - 4, o.width * p, 4)

  return ctx.getImageData(0, 0, o.width, o.height)
}

/** 同一张 canvas 的像素 → Rust ffmpeg，不存在第二套预览布局。 */
export function framePixels(canvas: HTMLCanvasElement): Uint8Array {
  const ctx = canvas.getContext('2d', { willReadFrequently: true })
  if (!ctx) return new Uint8Array()
  const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data
  return new Uint8Array(data.buffer.slice(0))
}

export function frameStats() {
  return { rendered, reused }
}
