//! §7 DSP：EQ 10 段双二阶 → SurroundChain → 30ms 淡变 → −1dBFS 限幅。
//! 待接入依赖：rustfft / realfft（2048 点频谱）、人声减弱矩阵、Adaptive REPET。
pub const STATUS: &str = "pending: biquad EQ + surround + vocal reduce";

pub trait DspChain {
    fn apply_eq(&mut self, bands: usize);
    fn apply_surround(&mut self, strength: f32);
    fn fade(&mut self, millis: u32);
}
