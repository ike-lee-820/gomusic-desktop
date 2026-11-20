//! §3 / §6.15 系统集成：托盘常驻、通知、防休眠、deep-link 回写、
//! 文件关联（命令行 / 单实例转发）、开机自启、录制进度同步。
use crate::audio::AudioEngine;
use serde_json::json;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Listener, Manager,
};
use tauri_plugin_notification::NotificationExt;
use tracing::warn;

static SLEEP_LOCK: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// 托盘常驻：显示主窗口 / 播放暂停 / 上一首 / 下一首 / 录制进度 / 退出。
pub fn setup_tray(app: &AppHandle, engine: Arc<AudioEngine>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let play = MenuItem::with_id(app, "play", "播放 / 暂停", true, None::<&str>)?;
    let prev = MenuItem::with_id(app, "prev", "上一首", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "下一首", true, None::<&str>)?;
    let progress = MenuItem::with_id(app, "rec-progress", "录制进度 —", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &play, &prev, &next, &progress, &sep, &quit])?;
    let _ = &progress;

    let eng = engine.clone();
    let mut builder = TrayIconBuilder::with_id("main");
    if let Some(icon) = app.default_window_icon() { builder = builder.icon(icon.clone()) }
    builder
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "play" => {
                let playing = eng.state().get("playing").and_then(|v| v.as_bool()).unwrap_or(false);
                if playing { eng.pause() } else { eng.play(None) }
            }
            "prev" => eng.prev(),
            "next" => eng.next(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle())
            }
        })
        .build(app)?;
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show(); let _ = w.set_focus();
    }
}

/// 三段式进度 → 托盘菜单文案 + 系统通知（页面走 Channel，录制列表读 SQLite）。
pub fn rec_progress(app: &AppHandle, p: &crate::models::RecProgress) {
    let text = format!("录制进度 {}% · {}", p.percent, p.detail);
    if let Some(tray) = app.tray_by_id("main") { let _ = tray.set_tooltip(Some(text.as_str())); }
    app.notification().builder()
        .title("GoMusic 录制中")
        .body(&format!("阶段 {} · {} · {}%", p.phase, p.detail, p.percent))
        .show().ok();
}

/// 防休眠：Windows SetThreadExecutionState / macOS caffeinate / Linux systemd-inhibit。
pub fn prevent_sleep(on: bool) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        extern "system" { fn SetThreadExecutionState(flags: u32) -> u32; }
        const ES_CONTINUOUS: u32 = 0x8000_0000;
        const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
        const ES_AWAYMODE_REQUIRED: u32 = 0x0000_0040;
        unsafe {
            SetThreadExecutionState(if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED } else { ES_CONTINUOUS });
        }
        return
    }
    #[allow(unreachable_code)]
    {
        let mut guard = SLEEP_LOCK.lock().unwrap();
        if on {
            if guard.is_some() { return }
            let child = if cfg!(target_os = "macos") {
                Command::new("caffeinate").args(["-d", "-i", "-s"]).spawn()
            } else {
                Command::new("systemd-inhibit")
                    .args(["--what=sleep", "--why=GoMusic recording", "--mode=block", "sleep", "2147483647"])
                    .spawn()
            };
            match child { Ok(c) => *guard = Some(c), Err(e) => warn!(target: "gomusic::sys", "prevent_sleep: {}", e) }
        } else if let Some(mut c) = guard.take() { let _ = c.kill(); }
    }
}

/// gomusic://verify?token=... 由系统浏览器验证完成后回调；token 只留在 Rust 侧。
pub fn setup_deep_link(app: &AppHandle, engine: Arc<AudioEngine>) {
    let app2 = app.clone();
    app.listen("deep-link://new-url", move |event| {
        let payload = event.payload();
        let text = serde_json::from_str::<serde_json::Value>(payload)
            .ok().and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| payload.to_string());
        let Some(pos) = text.find("gomusic://") else { return };
        let url = &text[pos..];
        if url.contains("verify") {
            let token = url.split('?').nth(1).unwrap_or("")
                .split('&').find_map(|kv| kv.strip_prefix("token=").or_else(|| kv.strip_prefix("cookie=")))
                .unwrap_or("").to_string();
            if !token.is_empty() {
                engine.set_cookie(token.clone());
                let _ = app2.emit("verify://token", json!({ "ok": true }));
            }
        }
        let _ = app2.emit("deep-link://open", json!({ "url": url }));
    });
}

/// 文件关联：启动参数与单实例转发都走同一事件，前端 local_meta 解析后直接按路径播放。
pub fn forward_open_files(app: &AppHandle, argv: Vec<String>) {
    let files: Vec<String> = argv.into_iter()
        .filter(|a| !a.starts_with('-') && !a.contains("gomusic://") && std::path::Path::new(a).exists())
        .collect();
    if !files.is_empty() { let _ = app.emit("file://open", json!({ "paths": files })); }
}

pub fn startup_args(app: &AppHandle) {
    forward_open_files(app, std::env::args().skip(1).collect());
}

/// 开机自启（tauri-plugin-autostart）。
pub fn set_autostart(app: &AppHandle, on: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let a = app.autolaunch();
    let _ = if on { a.enable() } else { a.disable() };
}
