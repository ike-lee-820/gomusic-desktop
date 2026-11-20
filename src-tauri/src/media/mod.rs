//! §3 系统媒体键：souvlaki → Windows SMTC / macOS NowPlayingInfoCenter / Linux MPRIS。
//! MediaControls 由专用线程持有（平台实现可能非 Send/UnwindSafe），主状态只留一个
//! mpsc Sender；系统媒体键事件在线程内直接驱动音频引擎，窗口失焦也有效。
use crate::audio::AudioEngine;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn};

enum Cmd {
    Metadata(String, String, String, i64),
    Playback(bool, i64),
}

pub struct MediaBridge {
    tx: Mutex<mpsc::Sender<Cmd>>,
}

impl MediaBridge {
    pub fn new(engine: Arc<AudioEngine>) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let (etx, erx) = mpsc::channel::<MediaControlEvent>();
        std::thread::spawn(move || {
            let config = PlatformConfig {
                dbus_name: "net.myocd.gomusic.desktop",
                display_name: "GoMusic",
                hwnd: None,
            };
            let mut controls = match MediaControls::new(config) {
                Ok(c) => Some(c),
                Err(e) => { warn!(target: "gomusic::media", "unavailable: {}", e); None }
            };
            if let Some(c) = controls.as_mut() {
                let sender = etx.clone();
                if let Err(e) = c.attach(move |ev| { let _ = sender.send(ev); }) {
                    warn!(target: "gomusic::media", "attach: {}", e);
                } else {
                    info!(target: "gomusic::media", "souvlaki media controls attached");
                }
            }
            loop {
                while let Ok(cmd) = rx.try_recv() {
                    if let Some(c) = controls.as_mut() {
                        match cmd {
                            Cmd::Metadata(title, artist, album, ms) => {
                                let _ = c.set_metadata(MediaMetadata {
                                    title: Some(&title), artist: Some(&artist), album: Some(&album),
                                    cover_url: None, duration: Some(Duration::from_millis(ms.max(0) as u64)),
                                });
                            }
                            Cmd::Playback(playing, ms) => {
                                let progress = Some(MediaPosition(Duration::from_millis(ms.max(0) as u64)));
                                let _ = c.set_playback(if playing { MediaPlayback::Playing { progress } } else { MediaPlayback::Paused { progress } });
                            }
                        }
                    }
                }
                while let Ok(ev) = erx.try_recv() {
                    match ev {
                        MediaControlEvent::Play | MediaControlEvent::Toggle => engine.play(None),
                        MediaControlEvent::Pause | MediaControlEvent::Stop => engine.pause(),
                        MediaControlEvent::Next => engine.next(),
                        MediaControlEvent::Previous => engine.prev(),
                        MediaControlEvent::SetPosition(MediaPosition(d)) => engine.seek(d.as_millis() as i64),
                        MediaControlEvent::SeekBy(_, d) => {
                            let cur = engine.state().get("positionMs").and_then(|v| v.as_i64()).unwrap_or(0);
                            engine.seek(cur + d.as_millis() as i64)
                        }
                        MediaControlEvent::SetVolume(v) => engine.set_volume(v as f32),
                        _ => {}
                    }
                }
                std::thread::sleep(Duration::from_millis(40));
            }
        });
        Arc::new(Self { tx: Mutex::new(tx) })
    }

    pub fn set_metadata(&self, title: &str, artist: &str, album: &str, duration_ms: i64) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(Cmd::Metadata(title.to_string(), artist.to_string(), album.to_string(), duration_ms));
        }
    }

    pub fn set_playback(&self, playing: bool, position_ms: i64) {
        if let Ok(tx) = self.tx.lock() { let _ = tx.send(Cmd::Playback(playing, position_ms)); }
    }
}
