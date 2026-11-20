//! 全局状态：凭据/cookie/配置只留在 Rust 侧；音频引擎、SQLite、录制管理器、媒体桥。
use crate::audio::AudioEngine;
use crate::db::Database;
use crate::media::MediaBridge;
use crate::models::Song;
use crate::rec::RecManager;
use crate::util;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::warn;

pub struct AppState {
    pub client: Client,
    pub base_url: String,
    pub cookie: Arc<Mutex<String>>,
    pub settings: Arc<Mutex<Value>>,
    pub queue: Arc<Mutex<Vec<Song>>>,
    pub history: Arc<Mutex<Vec<Song>>>,
    pub tasks: Arc<Mutex<Vec<crate::models::RecTask>>>,
    pub audio: Arc<AudioEngine>,
    pub db: Arc<Database>,
    pub rec: Arc<RecManager>,
    pub media: Arc<MediaBridge>,
}

const DEFAULTS: &str = r#"{
  "qualityLevel": "higher",
  "autoDowngrade": true,
  "fontSource": "SOURCE_HAN_SANS",
  "wght": 400,
  "trayKeepAlive": true,
  "proxy": "gh-proxy",
  "preventSleep": true,
  "mediaKeys": true,
  "volume": 1.0,
  "autostart": false,
  "showTrans": true
}"#;

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("GoMusicDesktop/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(15))
            .cookie_store(true)
            .build()?;
        let settings: Value = std::fs::read_to_string(util::data_dir().join("store.json"))
            .ok().and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::from_str(DEFAULTS).unwrap_or_else(|_| json!({})));
        let cookie = std::fs::read_to_string(util::data_dir().join("cookie"))
            .unwrap_or_default().trim().to_string();

        let audio = AudioEngine::new();
        let db = Arc::new(Database::open());
        let rec = Arc::new(RecManager::new(db.clone()));
        let media = MediaBridge::new(audio.clone());
        audio.set_cookie(cookie.clone());
        if let Some(l) = settings.get("qualityLevel").and_then(Value::as_str) { audio.set_level(l.to_string()) }
        if let Some(v) = settings.get("volume").and_then(Value::as_f64) { audio.set_volume(v as f32) }

        let history = db.list_history(500);
        Ok(Self {
            client,
            base_url: "https://mapi.myocd.de5.net".to_string(),
            cookie: Arc::new(Mutex::new(cookie)),
            settings: Arc::new(Mutex::new(settings)),
            queue: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(history)),
            tasks: Arc::new(Mutex::new(db.list_tasks())),
            audio, db, rec, media,
        })
    }

    pub fn cookie_value(&self) -> String {
        self.cookie.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn set_cookie(&self, value: &str) {
        if let Ok(mut c) = self.cookie.lock() { *c = value.to_string() }
        self.audio.set_cookie(value.to_string());
        let path = util::data_dir().join("cookie");
        if let Some(p) = path.parent() { let _ = std::fs::create_dir_all(p); }
        let _ = std::fs::write(path, value);
    }

    /// 原子写 store.json（confy 替代方案，键名对齐手机端 SharedPreferences）。
    pub fn save_settings(&self, snapshot: &Value) {
        let dir = util::data_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) { warn!(target: "gomusic::store", "mkdir: {}", e); return }
        let final_path = dir.join("store.json");
        let tmp = dir.join("store.json.tmp");
        if let Ok(s) = serde_json::to_string_pretty(snapshot) {
            if std::fs::write(&tmp, s).is_ok() { let _ = std::fs::rename(&tmp, &final_path); }
        }
    }
}
