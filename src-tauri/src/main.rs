#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod db;
mod dsp;
mod local;
mod media;
mod models;
mod net;
mod rec;
mod state;
mod sys;
mod util;

use serde_json::Value;
use state::AppState;
use tauri::{Emitter, Listener, Manager};

fn main() {
    let state = AppState::new().expect("failed to initialise GoMusic state");
    let engine = state.audio.clone();
    let media = state.media.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();
            engine.start(handle.clone());
            if let Err(e) = sys::setup_tray(&handle, engine.clone()) {
                tracing::warn!(target: "gomusic::sys", "tray: {}", e);
            }
            sys::setup_deep_link(&handle, engine.clone());
            sys::startup_args(&handle);

            // 播放态 → 系统媒体控制（SMTC / NowPlaying / MPRIS），窗口失焦仍有效。
            let m = media.clone();
            handle.listen("player://state", move |event| {
                if let Ok(v) = serde_json::from_str::<Value>(event.payload()) {
                    let song = v.get("song");
                    let title = song.and_then(|s| s.get("name")).and_then(Value::as_str).unwrap_or("GoMusic");
                    let artist = song.and_then(|s| s.get("artist")).and_then(Value::as_str).unwrap_or("");
                    let album = song.and_then(|s| s.get("album")).and_then(Value::as_str).unwrap_or("");
                    let dur = v.get("durationMs").and_then(Value::as_i64).unwrap_or(0);
                    let playing = v.get("playing").and_then(Value::as_bool).unwrap_or(false);
                    let pos = v.get("positionMs").and_then(Value::as_i64).unwrap_or(0);
                    m.set_metadata(title, artist, album, dur);
                    m.set_playback(playing, pos);
                }
            });

            // 应用启动自动检查更新：Rust 侧发起，gh-proxy 优先 + 403/WAF 回退直连。
            let client = app.state::<AppState>().client.clone();
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(doc) = net::check_update(&client, false).await {
                    let _ = h.emit("update://available", &doc);
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            sys::forward_open_files(app, argv)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_cli::init())
        // 只接受 minisign 签名更新；公钥在 tauri.conf.json，私钥仅存 CI Secrets。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::get_song_url,
            commands::probe_levels,
            commands::get_lyric,
            commands::album_desc,
            commands::artist_desc,
            commands::artist_detail,
            commands::song_detail,
            commands::translate,
            commands::login,
            commands::open_verify,
            commands::player_play,
            commands::player_pause,
            commands::player_seek,
            commands::player_next,
            commands::player_prev,
            commands::player_set_queue,
            commands::set_volume,
            commands::set_repeat,
            commands::player_state,
            commands::eq_apply,
            commands::surround_apply,
            commands::spectrum,
            commands::local_scan,
            commands::local_meta,
            commands::local_lyric,
            commands::local_cover,
            commands::history_list,
            commands::history_add,
            commands::history_clear,
            commands::settings_get,
            commands::settings_set,
            commands::font_list,
            commands::font_import,
            commands::image_palette,
            commands::image_save,
            commands::open_external,
            commands::reveal_path,
            commands::resource_path,
            commands::rec_probe_levels,
            commands::rec_push_frame,
            commands::rec_start,
            commands::rec_cancel,
            commands::rec_list,
            commands::rec_delete,
            commands::rec_clear,
            commands::rec_ffmpeg_available,
            commands::update_check,
            commands::update_notes,
            commands::update_install,
            commands::file_assoc_status,
            commands::set_autostart,
            commands::set_prevent_sleep,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GoMusic");
}
