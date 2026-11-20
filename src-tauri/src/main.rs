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

fn main() {
    let state = AppState::new().expect("failed to initialise GoMusic state");

    tauri::Builder::default()
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
