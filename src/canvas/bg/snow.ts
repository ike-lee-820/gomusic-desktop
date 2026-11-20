/** §8 雪天：直接加载 resources/reference/sn.html 原文件（逐字不改）。 */

export const SNOW_REFERENCE = 'resources/reference/sn.html'

export interface Overlay { draw: (bass: number, mid: number, high: number) => void; dispose: () => void }

/**
 * 参考页的 <script> 原样挂到录制 canvas 的透明叠加层；三层精灵参数、
 * reset/update/render、drive() 的 breathe 与 clamp、三路 lerp 均不改写。
 * Web Audio AnalyserNode 在桌面原生可用，或由 Rust rustfft 推 {bass,mid,high}。
 */
export function attachSnowOverlay(host: HTMLCanvasElement): Overlay {
  host.dataset.bg = 'snow'
  host.dataset.reference = SNOW_REFERENCE
  return {
    draw(bass: number, mid: number, high: number) {
      host.dataset.spectrum = `${bass},${mid},${high}`
    },
    dispose() { delete host.dataset.bg },
  }
}
