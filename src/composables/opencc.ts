/** §4.3 繁简转换：opencc-js 打包进前端本地 JS，避免运行时依赖 cdn.jsdelivr.net。 */
import * as OpenCC from 'opencc-js'

const tw2cn = OpenCC.Converter({ from: 'tw', to: 'cn' })
const cn2tw = OpenCC.Converter({ from: 'cn', to: 'tw' })

export const toSimplified = (s: string): string => tw2cn(s)
export const toTraditional = (s: string): string => cn2tw(s)
