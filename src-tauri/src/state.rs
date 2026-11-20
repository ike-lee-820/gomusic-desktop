use std::sync::{Arc, Mutex};
use reqwest::Client;
use serde_json::{json, Value};
use crate::models::Song;

/// 凭据、cookie、配置全部留在 Rust 侧，绝不进入前端 bundle。
pub struct AppState {
    pub client: Client,
    pub base_url: String,
    pub cookie: Arc<Mutex<String>>,
    pub settings: Arc<Mutex<Value>>,
    pub queue: Arc<Mutex<Vec<Song>>>,
    pub history: Arc<Mutex<Vec<Song>>>,
    pub tasks: Arc<Mutex<Vec<crate::models::RecTask>>>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("GoMusicDesktop/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(15))
            .cookie_store(true)
            .build()?;
        Ok(Self {
            client,
            base_url: "https://mapi.myocd.de5.net".to_string(),
            cookie: Arc::new(Mutex::new(String::new())),
            settings: Arc::new(Mutex::new(json!({
                "qualityLevel": "higher",
                "autoDowngrade": true,
                "fontSource": "SOURCE_HAN_SANS",
                "wght": 400,
                "trayKeepAlive": true,
                "proxy": "gh-proxy",
                "preventSleep": true,
                "mediaKeys": true
            }))),
            queue: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn cookie_value(&self) -> String {
        self.cookie.lock().map(|c| c.clone()).unwrap_or_default()
    }
}
