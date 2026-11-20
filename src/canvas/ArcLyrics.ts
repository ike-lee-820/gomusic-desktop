/** §6.3 圆弧歌词几何：圆心在歌词区左边缘外侧 0.7×区宽，半径 0.78×区宽。 */
export interface ArcLine {
  text: string
  angle: number
  tilt: number
  opacity: number
  wght: number
  current: boolean
}

const smoothstep = (x: number) => {
  const t = Math.min(1, Math.max(0, x))
  return t * t * (3 - 2 * t)
}

export interface ArcFrame {
  lines: ArcLine[]
  cx: number
  cy: number
  r: number
}

export function computeArcFrame(lines: string[], focus: number, width: number, height: number): ArcFrame {
  const cx = -0.7 * width
  const cy = height * 0.5
  const r = 0.78 * width
  const result: ArcLine[] = []
  for (let i = 0; i < lines.length; i++) {
    const delta = i - focus
    if (Math.abs(delta) > 4) continue
    const angle = delta * 0.26
    const k = smoothstep(Math.abs(angle) / 1.05)
    result.push({
      text: lines[i],
      angle,
      tilt: (20 * delta) / 4,
      opacity: 1 - k * 0.72,
      wght: 400 + (1 - k) * 200,
      current: delta === 0,
    })
  }
  return { lines: result, cx, cy, r }
}

export function drawArcFrame(
  ctx: CanvasRenderingContext2D,
  frame: ArcFrame,
  width: number,
  accent: string,
  fontSize = 22,
): void {
  ctx.clearRect(0, 0, width, ctx.canvas.height)
  for (const line of frame.lines) {
    const x = frame.cx + Math.cos(line.angle) * frame.r
    const y = frame.cy + Math.sin(line.angle) * frame.r
    ctx.save()
    ctx.translate(x, y)
    ctx.rotate((line.tilt * Math.PI) / 180)
    ctx.globalAlpha = line.opacity
    ctx.fillStyle = line.current ? accent : '#F2F4F8'
    ctx.font = `${Math.round(line.wght)} ${line.current ? fontSize + 4 : fontSize}px var(--go-font-lyric, serif)`
    ctx.textAlign = 'center'
    ctx.fillText(line.text.slice(0, 42), 0, 0, width * 0.78)
    ctx.restore()
  }
}
