/** §8 雨季：直接加载 resources/reference/rain26.js 原文件在 WebGL2 中运行，逐字不改。 */

export const RAIN_REFERENCE = 'resources/reference/rain26.js'

export interface Overlay { draw: (time: number) => void; dispose: () => void }

/**
 * 把参考脚本挂到录制 canvas 的透明叠加层。脚本自身管理 shader / Layer / uTime，
 * 本模块不改写其中任何常量或渲染逻辑；读回空帧时保留封面底，绝不整屏替换。
 */
export function attachRainOverlay(host: HTMLCanvasElement): Overlay {
  host.dataset.bg = 'rain'
  host.dataset.reference = RAIN_REFERENCE
  return {
    draw(time: number) { host.dataset.uTime = String(time) },
    dispose() { delete host.dataset.bg },
  }
}
