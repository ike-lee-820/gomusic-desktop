//! §3 系统集成：托盘常驻、防休眠、deep-link、文件关联、单实例、开机自启。
//! 防休眠：Windows SetThreadExecutionState / macOS caffeinate / Linux systemd-inhibit。
pub const STATUS: &str = "pending: tray + deep-link + single-instance + autostart";

pub trait SystemHooks {
    fn prevent_sleep(&mut self, on: bool);
    fn register_deep_link(&mut self, scheme: &str);
}
