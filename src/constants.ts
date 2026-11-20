/** §4.5 完整档位表，相邻回退，不跳档。与 Rust net::NM_LEVELS 同步。 */
export const NM_LEVELS = [
  'jymaster', 'vivid', 'dolby', 'jyeffect', 'hires',
  'lossless', 'exhigh', 'higher', 'standard',
] as const
