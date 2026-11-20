//! §3 媒体控制：souvlaki → Windows SMTC / macOS NowPlaying / Linux MPRIS。
//! 系统媒体键在窗口失焦时仍可用。待接入依赖：souvlaki。
pub const STATUS: &str = "pending: souvlaki media controls";

pub trait MediaControls {
    fn set_metadata(&mut self, title: &str, artist: &str);
    fn set_progress(&mut self, percent: u8);
}
