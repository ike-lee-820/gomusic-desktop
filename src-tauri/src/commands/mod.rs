//! §2.1 IPC 命令表：全部命令接入真实后端（net / audio / dsp / local / db / rec / media / sys）。
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{ipc::Channel, AppHandle, Manager, State};
use tracing::info;
use crate::models::*;
use crate::net;
use crate::state::AppState;
use crate::{dsp, local, rec, sys, util};

type R<T> = Result<T, String>;

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
    if force.unwrap_or(false) {
        // 点「暂无歌词」清空歌词缓存后重新拉取（force=true）。
        let p = util::cache_dir().join(format!("lyric-{id}.json"));
        let _ = std::fs::remove_file(p);
        info!(target: "gomusic::net", "lyric cache cleared for {}", id);
    }
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
pub async fn song_detail(state: State<'_, AppState>, ids: Vec<i64>) -> R<Value> {
    let ids = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    net::request(&state, "/song/detail", vec![("ids".into(), ids)], false).await
}

#[tauri::command]
pub async fn translate(state: State<'_, AppState>, text: String, from: Option<String>, to: Option<String>) -> R<String> {
    net::translate(&state, &text, &from.unwrap_or_else(|| "auto".into()), &to.unwrap_or_else(|| "zh-CN".into())).await
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>, phone: String, captcha: Option<String>, password: Option<String>) -> R<LoginState> {
    let mut params = vec![("phone".into(), phone.clone())];
    if let Some(c) = captcha { params.push(("captcha".into(), c)) }
    if let Some(p) = password { params.push(("password".into(), p)) }
    let body = net::request(&state, "/login/cellphone", params, false).await.map_err(|e| {
        if e.starts_with("RISK:") { "触发风控，请在系统浏览器完成验证".to_string() } else { "登录失败，请检查验证码或密码".to_string() }
    })?;
    let code = body.get("code").and_then(Value::as_i64).unwrap_or(200);
    if code != 200 {
        let msg = match code {
            -462 | 10004 | 460 => "触发风控，请在系统浏览器完成验证",
            10003 => "手机号已被复用，请更换手机号",
            _ => "登录失败，请检查验证码或密码",
        };
        return Err(msg.to_string())
    }
    let cookie = net::pick(&body, &["cookie", "token"]).and_then(|v| v.as_str()).unwrap_or("").trim_matches('"').to_string();
    state.set_cookie(&cookie);
    let profile = net::pick(&body, &["profile"]).cloned().unwrap_or(Value::Null);
    let logged = !cookie.is_empty() || profile.get("userId").is_some();
    if !logged { return Err("登录失败，请检查验证码或密码".into()) }
    Ok(LoginState {
        logged,
        user_id: profile.get("userId").and_then(Value::as_i64),
        nickname: profile.get("nickname").and_then(Value::as_str).map(str::to_string),
        avatar_url: profile.get("avatarUrl").and_then(Value::as_str).map(str::to_string),
    })
}

#[tauri::command]
pub async fn open_verify(verify_url: String) -> R<()> {
    info!(target: "gomusic::verify", "open risk verification in system browser: {}", verify_url);
    tauri_plugin_opener::open_url(&verify_url, None::<&str>).map_err(|e| e.to_string())
}

// ── 播放引擎（真实 cpal 输出，状态走 player://state 事件） ──────────
#[tauri::command] pub async fn player_play(state: State<'_, AppState>, uri: Option<String>) -> R<()> { state.audio.play(uri); Ok(()) }
#[tauri::command] pub async fn player_pause(state: State<'_, AppState>) -> R<()> { state.audio.pause(); Ok(()) }
#[tauri::command] pub async fn player_seek(state: State<'_, AppState>, position_ms: i64) -> R<()> { state.audio.seek(position_ms); Ok(()) }
#[tauri::command] pub async fn player_next(state: State<'_, AppState>) -> R<()> { state.audio.next(); Ok(()) }
#[tauri::command] pub async fn player_prev(state: State<'_, AppState>) -> R<()> { state.audio.prev(); Ok(()) }
#[tauri::command]
pub async fn player_set_queue(state: State<'_, AppState>, songs: Vec<Song>) -> R<()> {
    if let Ok(mut q) = state.queue.lock() { *q = songs.clone() }
    state.audio.set_queue(songs);
    Ok(())
}
#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> R<()> {
    state.audio.set_volume(volume);
    if let Ok(mut s) = state.settings.lock() { s["volume"] = json!(volume) }
    state.save_settings(&state.settings.lock().unwrap().clone());
    Ok(())
}
#[tauri::command] pub async fn set_repeat(state: State<'_, AppState>, mode: u8) -> R<()> { state.audio.set_repeat(mode); Ok(()) }
#[tauri::command] pub async fn player_state(state: State<'_, AppState>) -> R<Value> { Ok(state.audio.state()) }

// ── DSP（解码后 PCM 上的真实双二阶链 / 环绕链） ──────────────────────
#[tauri::command]
pub async fn eq_apply(state: State<'_, AppState>, config: EqConfig) -> R<()> {
    state.audio.apply_eq(config.clone());
    if let Ok(mut s) = state.settings.lock() { s["eq"] = serde_json::to_value(&config).unwrap_or(Value::Null) }
    state.save_settings(&state.settings.lock().unwrap().clone());
    Ok(())
}
#[tauri::command]
pub async fn surround_apply(state: State<'_, AppState>, config: SurroundConfig) -> R<()> {
    state.audio.apply_surround(config.clone());
    if let Ok(mut s) = state.settings.lock() { s["surround"] = serde_json::to_value(&config).unwrap_or(Value::Null) }
    state.save_settings(&state.settings.lock().unwrap().clone());
    Ok(())
}

/// rustfft 2048 点频谱（雪天音频响应），供前端叠层取 {bass,mid,high}。
#[tauri::command]
pub async fn spectrum(samples: Vec<f32>, sample_rate: Option<u32>) -> R<[f32; 3]> {
    let mut s = dsp::Spectrum::default();
    Ok(s.analyze(&samples, sample_rate.unwrap_or(48_000)))
}

// ── 本地音乐（walkdir + lofty，无网络请求） ──────────────────────────
#[tauri::command]
pub async fn local_scan(state: State<'_, AppState>, paths: Vec<String>) -> R<Vec<Song>> {
    let songs = local::scan(&paths);
    if let Ok(mut q) = state.queue.lock() { *q = songs.clone() }
    Ok(songs)
}
#[tauri::command]
pub async fn local_meta(path: String) -> R<Song> { Ok(local::meta(std::path::Path::new(&path))) }
#[tauri::command]
pub async fn local_lyric(path: String) -> R<Lyric> { Ok(local::lyric(std::path::Path::new(&path))) }
#[tauri::command]
pub async fn local_cover(path: String) -> R<Option<String>> {
    let Some(bytes) = local::cover(std::path::Path::new(&path)) else { return Ok(None) };
    let dir = util::cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let ext = if bytes.starts_with(b"\x89PNG") { "png" } else { "jpg" };
    let name = format!("cover-{}.{}", local::meta(std::path::Path::new(&path)).id, ext);
    let p = dir.join(name);
    std::fs::write(&p, &bytes).map_err(|e| e.to_string())?;
    Ok(Some(p.to_string_lossy().to_string()))
}

// ── 历史（SQLite，在线 + 本地统一） ──────────────────────────────────
#[tauri::command]
pub async fn history_list(state: State<'_, AppState>, limit: Option<i64>) -> R<Vec<Song>> {
    Ok(state.db.list_history(limit.unwrap_or(100)))
}
#[tauri::command]
pub async fn history_add(state: State<'_, AppState>, song: Song) -> R<()> {
    state.db.add_history(&song);
    if let Ok(mut h) = state.history.lock() { h.insert(0, song); h.truncate(500) }
    Ok(())
}
#[tauri::command]
pub async fn history_clear(state: State<'_, AppState>) -> R<()> {
    state.db.clear_history();
    if let Ok(mut h) = state.history.lock() { h.clear() }
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
    if let Some(obj) = kv.as_object() {
        let mut s = state.settings.lock().map_err(|_| "settings lock".to_string())?;
        for (k, v) in obj { s[k] = v.clone() }
        if let Some(l) = s.get("qualityLevel").and_then(Value::as_str) { state.audio.set_level(l.to_string()) }
        if let Some(v) = s.get("volume").and_then(Value::as_f64) { state.audio.set_volume(v as f32) }
        let snap = s.clone();
        drop(s);
        state.save_settings(&snap);
    }
    Ok(())
}
#[tauri::command]
pub async fn font_list() -> R<Vec<String>> {
    let mut out = vec!["系统默认".to_string(), "思源宋体 VF".to_string(), "思源黑体 VF".to_string()];
    let dir = util::data_dir().join("fonts");
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if n.ends_with(".otf") || n.ends_with(".ttf") || n.ends_with(".woff2") { out.push(n.to_string()) }
            }
        }
    }
    Ok(out)
}
#[tauri::command]
pub async fn font_import(path: String) -> R<String> {
    let src = std::path::Path::new(&path);
    let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("imported.otf").to_string();
    let dir = util::data_dir().join("fonts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dst = dir.join(&name);
    std::fs::copy(src, &dst).map_err(|e| e.to_string())?;
    Ok(dst.to_string_lossy().to_string())
}

/// 下载封面 + 三级缓存（内存/磁盘/URL）+ Palette 主色（HSL 钳制）。
#[tauri::command]
pub async fn image_palette(state: State<'_, AppState>, url: String) -> R<String> {
    let key = format!("cover:{url}");
    let bytes = if let Some((p, _)) = state.db.cache_get(&key) {
        std::fs::read(p).unwrap_or_default()
    } else {
        let b: Vec<u8> = state.client.get(url.as_str()).send().await.map_err(|e| e.to_string())?
            .bytes().await.map_err(|e| e.to_string())?.to_vec();
        let dir = util::cache_dir();
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join(format!("cover-{}.jpg", url.len()));
        let _ = std::fs::write(&p, &b);
        state.db.cache_put(&key, &p.to_string_lossy(), "cover", b.len() as i64);
        b
    };
    Ok(palette_hex(&bytes))
}

fn palette_hex(bytes: &[u8]) -> String {
    let img = match image::load_from_memory(bytes) { Ok(i) => i.thumbnail(48, 48).to_rgb8(), Err(_) => return "#6ea8fe".into() };
    let (mut r, mut g, mut b, mut n) = (0f64, 0f64, 0f64, 0f64);
    for px in img.pixels() {
        let (rf, gf, bf) = (px[0] as f64 / 255.0, px[1] as f64 / 255.0, px[2] as f64 / 255.0);
        let max = rf.max(gf).max(bf); let min = rf.min(gf).min(bf);
        let s = if max - min < 1e-6 { 0.0 } else { (max - min) / (1.0 - (2.0 * (max + min) / 2.0 - 1.0).abs()).max(1e-6) };
        if s > 0.25 && max > 0.25 && max < 0.92 { r += rf; g += gf; b += bf; n += 1.0 }
    }
    if n == 0.0 { return "#6ea8fe".into() }
    let (r, g, b) = (r / n, g / n, b / n);
    let max = r.max(g).max(b); let min = r.min(g).min(b); let l = (max + min) / 2.0;
    let mut h = if (max - min).abs() < 1e-6 { 0.0 } else if max == r { (g - b) / (max - min) } else if max == g { 2.0 + (b - r) / (max - min) } else { 4.0 + (r - g) / (max - min) };
    h = (h / 6.0).rem_euclid(1.0);
    let s = 0.60f64;   // 钳 0.45–0.75
    let l = l.clamp(0.52, 0.68);
    // HSL → RGB
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (rr, gg, bb) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    format!("#{:02x}{:02x}{:02x}", ((rr + m) * 255.0) as u8, ((gg + m) * 255.0) as u8, ((bb + m) * 255.0) as u8)
}

#[tauri::command]
pub async fn image_save(state: State<'_, AppState>, url: String, name: String) -> R<String> {
    let bytes: Vec<u8> = state.client.get(url.as_str()).send().await.map_err(|e| e.to_string())?
        .bytes().await.map_err(|e| e.to_string())?.to_vec();
    let dir = util::download_dir().join("图片");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let ext = if url.contains(".png") { "png" } else { "jpg" };
    let path = dir.join(format!("{safe}.{ext}"));
    tokio::fs::write(&path, &bytes).await.map_err(|e| e.to_string())?;
    state.db.cache_put(&format!("saved:{url}"), &path.to_string_lossy(), "image", bytes.len() as i64);
    Ok(path.to_string_lossy().to_string())
}
#[tauri::command]
pub async fn open_external(target: String) -> R<()> {
    info!(target: "gomusic::opener", "{}", target);
    tauri_plugin_opener::open_url(&target, None::<&str>).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn reveal_path(path: String) -> R<()> { rec::reveal(&path); Ok(()) }

/// resources/reference/ 下参考文件只读原样运行；返回绝对路径供 convertFileSrc 加载。
#[tauri::command]
pub async fn resource_path(app: AppHandle, name: String) -> R<String> {
    use tauri::path::BaseDirectory;
    app.path().resolve(&name, BaseDirectory::Resource)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

// ── 录制（真实编码 + SQLite 快照 + 托盘/通知同步） ──────────────────
#[tauri::command]
pub async fn rec_probe_levels(state: State<'_, AppState>, id: i64) -> R<Vec<String>> {
    Ok(net::probe_levels(&state, id).await)
}
#[tauri::command]
pub async fn rec_push_frame(state: State<'_, AppState>, rgba: Vec<u8>, pts_us: u64) -> R<()> {
    state.rec.push_frame(rgba, pts_us);
    Ok(())
}
#[tauri::command]
pub async fn rec_start(
    app: AppHandle,
    state: State<'_, AppState>,
    config: RecConfig,
    on_progress: Channel<RecProgress>,
) -> R<()> {
    let song = state.audio.current_song();
    let hook: Arc<dyn Fn(&RecProgress) + Send + Sync> = Arc::new(move |p| sys::rec_progress(&app, p));
    sys::prevent_sleep(true);
    let result = state.rec.run(&*state, config, song, on_progress, Some(hook)).await;
    sys::prevent_sleep(false);
    result.map(|_| ())
}
#[tauri::command]
pub async fn rec_cancel(state: State<'_, AppState>, task_id: String) -> R<()> { state.rec.cancel(&task_id); Ok(()) }
#[tauri::command]
pub async fn rec_list(state: State<'_, AppState>) -> R<Vec<RecTask>> {
    let list = state.rec.list();
    if let Ok(mut t) = state.tasks.lock() { *t = list.clone() }
    Ok(list)
}
#[tauri::command]
pub async fn rec_delete(state: State<'_, AppState>, task_id: String) -> R<()> { state.rec.delete(&task_id); Ok(()) }
#[tauri::command]
pub async fn rec_clear(state: State<'_, AppState>) -> R<()> { state.db.clear_tasks(); Ok(()) }
#[tauri::command]
pub async fn rec_ffmpeg_available() -> R<bool> { Ok(rec::ffmpeg_available()) }

// ── 更新 / 系统集成 ─────────────────────────────────────────
#[tauri::command]
pub async fn update_check(state: State<'_, AppState>, force: Option<bool>) -> R<UpdateDoc> {
    net::check_update(&state.client, force.unwrap_or(false)).await
}
#[tauri::command]
pub async fn update_notes() -> R<UpdateDoc> {
    Ok(UpdateDoc {
        version: env!("CARGO_PKG_VERSION").to_string(),
        body: crate::models::OFFLINE_CHANGELOG.to_string(),
        ..Default::default()
    })
}
/// tauri-plugin-updater：读取 latest.json，校验 minisign 签名后下载安装；未签名更新直接拒绝。
#[tauri::command]
pub async fn update_install(app: AppHandle) -> R<()> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    match update {
        Some(u) => {
            u.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
            Ok(())
        }
        None => Err("已是最新版本".into()),
    }
}

#[tauri::command]
pub async fn file_assoc_status() -> R<Vec<String>> {
    Ok(vec!["mp3", "flac", "m4a", "aac", "ape", "wav", "ogg", "oga", "wma", "aiff", "aif", "dsf", "dff", "lrc"]
        .into_iter().map(str::to_string).collect())
}
#[tauri::command]
pub async fn set_autostart(app: AppHandle, state: State<'_, AppState>, on: bool) -> R<()> {
    sys::set_autostart(&app, on);
    if let Ok(mut s) = state.settings.lock() { s["autostart"] = json!(on) }
    state.save_settings(&state.settings.lock().unwrap().clone());
    Ok(())
}
#[tauri::command]
pub async fn set_prevent_sleep(on: bool) -> R<()> { sys::prevent_sleep(on); Ok(()) }
