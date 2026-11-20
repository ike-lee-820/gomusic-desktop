//! §9 SQLite：播放历史（在线+本地统一）、录制任务与快照、cache 索引。
//! 待接入依赖：rusqlite / tauri-plugin-sql。进程重启按快照续跑录制。
pub const STATUS: &str = "pending: sqlite history + record snapshots";

pub trait Store {
    fn add_history(&mut self, id: i64);
    fn snapshot(&self) -> String;
}
