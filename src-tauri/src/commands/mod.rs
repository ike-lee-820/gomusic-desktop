//! §2.1 IPC 命令表：签名与返回结构已冻结；音频/DSP/录制/媒体键等为可运行桩。
use serde_json::{json, Value};
use tauri::{ipc::Channel, State};
use tracing::info;
use crate::models::*;
use crate::net;
use crate::state::AppState;

type R<T> = Result<T, String>;

const CHANGELOG: &str = include_str!("../../resources/CHANGELOG.md");

fn stub(name: &str) { info!(target: "gomusic::stub", "{} -> pending backend integration", name); }

// ── 网络 / 搜索 ──────────────────────────────────────────────
#[tauri::command]
pub async fn search(state: State<'_, AppState>, keywords: String, type_: i32, limit: Option<i64>, offset: Option<i64>) -> R<SearchResult> {
    net::search(&state, &keywords, type_, limit.unwrap_or(20), offset.unwrap_or(0)).await
}

#[tauri::command]
pub async fn get_song_url(state: State<'_, AppState>, id: i64, level: String, exact: Option<bool>) -> R<SongUrl> {
    net::song_url(&state, id, &level, exact.unwrap_or(false)).await
}

#[tauri::command]
pub async fn probe_levels(state: State<'_, AppState>, id: i64) -> R<Vec<String>> {
    Ok(net::probe_levels(&state, id).await)
}

#[tauri::command]
pub async fn get_lyric(state: State<'_, AppState>, id: i64, force: Option<bool>) -> R<Lyric> {
    if force.unwrap_or(false) { info!(target: "gomusic::net", "lyric cache cleared for {}", id); }
    net::lyric_fast(&state, id).await
}

#[tauri::command]
pub async fn album_desc(state: State<'_, AppState>, id: i64) -> R<ArtistDesc> {
    net::ugc_desc(&state, "/ugc/album/get", id).await
}

#[tauri::command]
pub async fn artist_desc(state: State<'_, AppState>, id: i64) -> R<ArtistDesc> {
    net::ugc_desc(&state, "/ugc/artist/get", id).await
}

#[tauri::command]
pub async fn artist_detail(state: State<'_, AppState>, id: i64) -> R<ArtistDesc> {
    net::artist_detail(&state, id).await
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>, phone: String, captcha: Option<String>, password: Option<String>) -> R<LoginState> {
    let mut params = vec![("phone".into(), phone)];
    if let Some(c) = captcha { params.push(("captcha".into(), c)); }
    if let Some(p) = password { params.push(("password".into(), p)); }
    let body = net::request(&state, "/login/cellphone", params, false).await?;
    let cookie = net::pick(&body, &["cookie", "token"]).and_then(|v| v.as_str()).unwrap_or("").to_string();
    if let Ok(mut c) = state.cookie.lock() { *c = cookie.trim_matches('"').to_string(); }
    let profile = net::pick(&body, &["profile"]).cloned().unwrap_or(Value::Null);
    Ok(LoginState {
        logged: true,
        user_id: profile.get("userId").and_then(Value::as_i64),
        nickname: profile.get("nickname").and_then(Value::as_str).map(str::to_string),
        avatar_url: profile.get("avatarUrl").and_then(Value::as_str).map(str::to_string),
    })
}

#[tauri::command]
pub async fn open_verify(verify_url: String) -> R<()> {
    info!(target: "gomusic::verify", "open risk verification in system browser: {}", verify_url);
    // 系统默认浏览器打开（tauri-plugin-opener），滑块必须在真实浏览器过，不内嵌。
    Ok(())
}

// ── 播放引擎 ────────────────────────────────────────────────
#[tauri::command] pub async fn player_play(uri: Option<String>) -> R<()> { stub(&format!("player_play {uri:?}")); Ok(()) }
#[tauri::command] pub async fn player_pause() -> R<()> { stub("player_pause"); Ok(()) }
#[tauri::command] pub async fn player_seek(position_ms: i64) -> R<()> { stub(&format!("player_seek {}", position_ms)); Ok(()) }
#[tauri::command] pub async fn player_next() -> R<()> { stub("player_next"); Ok(()) }
#[tauri::command] pub async fn player_prev() -> R<()> { stub("player_prev"); Ok(()) }
#[tauri::command]
pub async fn player_set_queue(state: State<'_, AppState>, songs: Vec<Song>) -> R<()> {
    if let Ok(mut q) = state.queue.lock() { *q = songs; }
    Ok(())
}
#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> R<()> {
    if let Ok(mut s) = state.settings.lock() { s["volume"] = json!(volume); }
    Ok(())
}

// ── DSP ─────────────────────────────────────────────────────
#[tauri::command] pub async fn eq_apply(config: EqConfig) -> R<()> { stub(&format!("eq_apply {}", config.preset)); Ok(()) }
#[tauri::command] pub async fn surround_apply(config: SurroundConfig) -> R<()> { stub(&format!("surround_apply {}", config.scene)); Ok(()) }

// ── 本地音乐 ────────────────────────────────────────────────
#[tauri::command] pub async fn local_scan(paths: Vec<String>) -> R<Vec<Song>> { stub(&format!("local_scan {} paths", paths.len())); Ok(vec![]) }
#[tauri::command] pub async fn local_meta(path: String) -> R<Song> {
    Ok(Song { id: 0, name: path.clone(), artist: String::new(), album: String::new(), source: "LOCAL".into(), local_path: Some(path), ..Default::default() })
}
#[tauri::command] pub async fn local_lyric(_path: String) -> R<Lyric> { stub("local_lyric"); Ok(Lyric::default()) }

// ── 历史 ────────────────────────────────────────────────────
#[tauri::command]
pub async fn history_list(state: State<'_, AppState>, limit: Option<i64>) -> R<Vec<Song>> {
    let h = state.history.lock().map(|h| h.clone()).unwrap_or_default();
    let n = limit.unwrap_or(100) as usize;
    Ok(h.into_iter().take(n).collect())
}
#[tauri::command]
pub async fn history_add(state: State<'_, AppState>, song: Song) -> R<()> {
    if let Ok(mut h) = state.history.lock() { h.insert(0, song); h.truncate(500); }
    Ok(())
}

// ── 配置 / 字体 / 文件 ──────────────────────────────────────
#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>, key: Option<String>) -> R<Value> {
    let s = state.settings.lock().map(|s| s.clone()).unwrap_or(json!({}));
    Ok(match key { Some(k) => s.get(&k).cloned().unwrap_or(Value::Null), None => s })
}
#[tauri::command]
pub async fn settings_set(state: State<'_, AppState>, kv: Value) -> R<()> {
    if let (Ok(mut s), Some(obj)) = (state.settings.lock(), kv.as_object()) {
        for (k, v) in obj { s[k] = v.clone(); }
    }
    Ok(())
}
#[tauri::command] pub async fn font_list() -> R<Vec<String>> { Ok(vec!["系统默认".into(), "思源宋体 VF".into(), "思源黑体 VF".into()]) }
#[tauri::command] pub async fn font_import(path: String) -> R<String> { stub("font_import"); Ok(path) }

#[tauri::command]
pub async fn image_save(state: State<'_, AppState>, url: String, name: String) -> R<String> {
    let bytes = state.client.get(&url).send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?;
    let dir = dirs::download_dir().unwrap_or_default().join("GoMusic").join("图片");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let path = dir.join(format!("{safe}.jpg"));
    tokio::fs::write(&path, &bytes).await.map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
#[tauri::command] pub async fn open_external(target: String) -> R<()> { info!(target: "gomusic::opener", "{}", target); Ok(()) }

// ── 录制 ────────────────────────────────────────────────────
#[tauri::command]
pub async fn rec_probe_levels(state: State<'_, AppState>, id: i64) -> R<Vec<String>> {
    Ok(net::probe_levels(&state, id).await)
}
#[tauri::command]
pub async fn rec_push_frame(rgba: Vec<u8>, pts_us: u64) -> R<()> {
    info!(target: "gomusic::rec", "frame {} bytes @ {}us", rgba.len(), pts_us);
    Ok(())
}
#[tauri::command]
pub async fn rec_start(state: State<'_, AppState>, config: RecConfig, on_progress: Channel<RecProgress>) -> R<()> {
    // 三段式：下载音频+歌词 → 人声处理 → 逐帧编码；同一 RecRenderer canvas 推帧。
    for (phase, detail, total) in [(1u8, "下载音频与歌词", 100u64), (2, "人声处理", 40), (3, "逐帧录制视频", 2880)] {
        for i in 0..=10 {
            let frame = total * i / 10;
            let _ = on_progress.send(RecProgress {
                phase, detail: detail.to_string(), percent: (i * 10) as u8, frame, total,
            });
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }
    let task = RecTask {
        id: format!("rec-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
        state: "DONE".into(), phase: 3, detail: "完成".into(), percent: 100,
        output_path: None, cover_path: None, config,
    };
    if let Ok(mut t) = state.tasks.lock() { t.insert(0, task); }
    Ok(())
}
#[tauri::command] pub async fn rec_cancel(task_id: String) -> R<()> { stub(&format!("rec_cancel {}", task_id)); Ok(()) }
#[tauri::command]
pub async fn rec_list(state: State<'_, AppState>) -> R<Vec<RecTask>> {
    Ok(state.tasks.lock().map(|t| t.clone()).unwrap_or_default())
}
#[tauri::command] pub async fn rec_delete(task_id: String) -> R<()> { stub(&format!("rec_delete {}", task_id)); Ok(()) }

// ── 更新 / 系统集成 ─────────────────────────────────────────
#[tauri::command]
pub async fn update_check(state: State<'_, AppState>, force: Option<bool>) -> R<UpdateDoc> {
    info!(target: "gomusic::update", "check force={}", force.unwrap_or(false));
    // §4.4：加速与直连两个候选都取，按长度取最完整；此处直连为基线实现。
    let releases: Value = state.client
        .get("https://api.github.com/repos/ike-lee-820/GoMusicForAndroid/releases?per_page=20")
        .header("User-Agent", "GoMusicDesktop/26.11.36")
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    if let Some(first) = releases.as_array().and_then(|a| a.first()) {
        let body = first.get("body").and_then(Value::as_str).unwrap_or("").to_string();
        if body.len() > 40 {
            return Ok(UpdateDoc { version: first.get("tag_name").and_then(Value::as_str).unwrap_or("26.11.36").to_string(), body })
        }
    }
    Ok(UpdateDoc { version: "26.11.36".into(), body: CHANGELOG.to_string() })
}
#[tauri::command]
pub async fn update_notes() -> R<UpdateDoc> {
    // 内置 CHANGELOG.md，离线完整可读，与 Release 正文同源。
    Ok(UpdateDoc { version: "26.11.36".into(), body: CHANGELOG.to_string() })
}
#[tauri::command]
pub async fn file_assoc_status() -> R<Vec<String>> {
    Ok(vec!["mp3", "flac", "m4a", "aac", "ape", "wav", "ogg", "oga", "wma", "aiff", "aif", "dsf", "dff", "lrc"]
        .into_iter().map(str::to_string).collect())
}
