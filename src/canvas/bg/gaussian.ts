/** §8 高斯模糊：单张封面帧做一次轻微模糊，保留轮廓。 */
export function drawGaussian(
  ctx: CanvasRenderingContext2D,
  cover: HTMLImageElement,
  w: number,
  h: number,
): void {
  ctx.save()
  ctx.filter = 'blur(10px)'
  const scale = Math.max(w / cover.width, h / cover.height)
  const dw = cover.width * scale
  const dh = cover.height * scale
  ctx.drawImage(cover, (w - dw) / 2, (h - dh) / 2, dw, dh)
  ctx.restore()
}
