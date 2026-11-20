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

use state::AppState;
use tauri::{Emitter, Manager};

fn main() {
    let state = AppState::new().expect("failed to initialise GoMusic state");

    tauri::Builder::default()
        .setup(|app| {
            // 应用启动自动检查更新：默认 gh-proxy 加速，403/WAF 自动回退直连。
            // 检查在 Rust 侧进行，凭据不进 JS；结果以事件广播，窗口最小化也能收到。
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(st) = AppState::new() {
                    if let Ok(doc) = net::check_update(&st, false).await {
                        let _ = handle.emit("update://available", &doc);
                    }
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
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::get_song_url,
            commands::probe_levels,
            commands::get_lyric,
            commands::album_desc,
            commands::artist_desc,
            commands::artist_detail,
            commands::login,
            commands::open_verify,
            commands::player_play,
            commands::player_pause,
            commands::player_seek,
            commands::player_next,
            commands::player_prev,
            commands::player_set_queue,
            commands::set_volume,
            commands::eq_apply,
            commands::surround_apply,
            commands::local_scan,
            commands::local_meta,
            commands::local_lyric,
            commands::history_list,
            commands::history_add,
            commands::settings_get,
            commands::settings_set,
            commands::font_list,
            commands::font_import,
            commands::image_save,
            commands::open_external,
            commands::rec_probe_levels,
            commands::rec_push_frame,
            commands::rec_start,
            commands::rec_cancel,
            commands::rec_list,
            commands::rec_delete,
            commands::update_check,
            commands::update_notes,
            commands::file_assoc_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GoMusic");
}
