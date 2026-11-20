//! §9 SQLite（rusqlite bundled，无系统库依赖）：
//! 播放历史（在线 + 本地统一）、录制任务与快照、缓存索引。
//! 进程被强制终止后按快照续跑录制。
use crate::models::{RecTask, Song};
use crate::util;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::warn;

pub struct Database {
    conn: Mutex<Connection>,
}

fn db_path() -> PathBuf {
    let dir = util::data_dir().join("data");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("gomusic.db")
}

impl Database {
    pub fn open() -> Self {
        let conn = Connection::open(db_path()).unwrap_or_else(|_| Connection::open_in_memory().unwrap());
        conn.execute_batch(r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song TEXT NOT NULL,
                played_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS records (
                id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cache (
                key TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                level TEXT NOT NULL,
                size INTEGER NOT NULL
            );
        "#).ok();
        Self { conn: Mutex::new(conn) }
    }

    fn now() -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64
    }

    pub fn add_history(&self, song: &Song) {
        let json = serde_json::to_string(song).unwrap_or_default();
        if let Ok(c) = self.conn.lock() {
            let _ = c.execute("INSERT INTO history(song, played_at) VALUES(?1, ?2)", params![json, Self::now()]);
            let _ = c.execute("DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 500)", []);
        }
    }

    pub fn list_history(&self, limit: i64) -> Vec<Song> {
        let c = self.conn.lock().ok();
        let mut out = Vec::new();
        if let Some(c) = c {
            if let Ok(mut stmt) = c.prepare("SELECT song FROM history ORDER BY id DESC LIMIT ?1") {
                if let Ok(rows) = stmt.query_map(params![limit], |r| r.get::<_, String>(0)) {
                    for row in rows.flatten() {
                        if let Ok(s) = serde_json::from_str::<Song>(&row) { out.push(s) }
                    }
                }
            }
        }
        out
    }

    pub fn clear_history(&self) {
        if let Ok(c) = self.conn.lock() { let _ = c.execute("DELETE FROM history", []); }
    }

    pub fn save_task(&self, task: &RecTask) {
        let json = serde_json::to_string(task).unwrap_or_default();
        if let Ok(c) = self.conn.lock() {
            let _ = c.execute("INSERT OR REPLACE INTO records(id, task, updated_at) VALUES(?1, ?2, ?3)",
                params![task.id, json, Self::now()]);
        }
    }

    pub fn list_tasks(&self) -> Vec<RecTask> {
        let mut out = Vec::new();
        if let Ok(c) = self.conn.lock() {
            if let Ok(mut stmt) = c.prepare("SELECT task FROM records ORDER BY updated_at DESC") {
                if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                    for row in rows.flatten() {
                        if let Ok(t) = serde_json::from_str::<RecTask>(&row) { out.push(t) }
                    }
                }
            }
        }
        out.sort_by(|a, b| {
            let ar = a.state == "RUNNING"; let br = b.state == "RUNNING";
            br.cmp(&ar) // 进行中置顶
        });
        out
    }

    pub fn delete_task(&self, id: &str) {
        if let Ok(c) = self.conn.lock() { let _ = c.execute("DELETE FROM records WHERE id=?1", params![id]); }
    }

    pub fn clear_tasks(&self) {
        if let Ok(c) = self.conn.lock() { let _ = c.execute("DELETE FROM records", []); }
    }

    /// 启动时把上次未完成的快照标记为可续跑（state=PAUSED），供录制列表展示。
    pub fn resume_snapshots(&self) -> Vec<RecTask> {
        let mut out = self.list_tasks();
        for t in &mut out {
            if t.state == "RUNNING" {
                t.state = "PAUSED".into();
                t.detail = "进程中断，可续跑".into();
                self.save_task(t);
            }
        }
        out
    }

    pub fn cache_put(&self, key: &str, path: &str, level: &str, size: i64) {
        if let Ok(c) = self.conn.lock() {
            let _ = c.execute("INSERT OR REPLACE INTO cache(key, path, level, size) VALUES(?1,?2,?3,?4)",
                params![key, path, level, size]);
        }
    }

    pub fn cache_get(&self, key: &str) -> Option<(String, String)> {
        if let Ok(c) = self.conn.lock() {
            c.query_row("SELECT path, level FROM cache WHERE key=?1", params![key],
                |r| Ok((r.get(0)?, r.get(1)?))).ok()
        } else { None }
    }
}

pub fn init() -> Database {
    let db = Database::open();
    let pending = db.resume_snapshots();
    if !pending.is_empty() { warn!(target: "gomusic::db", "resumed {} record snapshots", pending.len()); }
    db
}
