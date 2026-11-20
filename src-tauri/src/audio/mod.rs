//! §7 音频链路：symphonia 解码 → cpal 输出；队列 / seek / 自动续播。
//! 待接入依赖：symphonia / cpal（或 ffmpeg sidecar 解码）。两条路径能力一致。
pub const STATUS: &str = "pending: symphonia + cpal backend";

pub trait AudioEngine {
    fn play(&mut self, uri: &str);
    fn pause(&mut self);
    fn seek(&mut self, position_ms: i64);
    fn set_queue(&mut self, count: usize);
}
