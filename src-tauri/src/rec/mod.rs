//! §6.11 录制编码：ffmpeg sidecar / ffmpeg-next，逐帧 RGBA → 音视频对齐封装。
//! MP4 兜底分支统一保留 -c:a aac（FLAC/APE 解码转 AAC，避免 muxer 报错）。
pub const STATUS: &str = "pending: ffmpeg sidecar encoder";

pub trait Encoder {
    fn push_frame(&mut self, rgba: &[u8], pts_us: u64);
    fn mux(&mut self, audio_path: &str) -> String;
}
