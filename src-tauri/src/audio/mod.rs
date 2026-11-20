//! §7 音频引擎（真实实现，非桩）：
//! ffmpeg sidecar 优先解码（覆盖 DSD/APE/WMA 等），缺失时纯 Rust symphonia 接管；
//! 解码 PCM → EQ 双二阶 → SurroundChain → 30ms 淡变 → −1dBFS 限幅 → cpal 输出。
//! 队列、精确 seek、自动续播、播放态事件 `player://state` 全部在 Rust 侧完成，
//! 与 Vue 组件生命周期 / ctrlOpen 完全解耦。
use crate::dsp::{soft_limit, EqChain, Fade, SurroundChain};
use crate::models::{EqConfig, Song, SurroundConfig};
use crate::util;
use serde_json::json;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

const SR: u32 = 48_000;
const CAP: usize = SR as usize * 2 * 2; // 2 秒立体声

#[derive(Default)]
struct Control {
    queue: Vec<Song>,
    index: usize,
    playing: bool,
    volume: f32,
    position_ms: i64,
    duration_ms: i64,
    seek_ms: Option<i64>,
    uri_override: Option<String>,
    repeat: u8, // 0 列表循环 / 1 单曲循环 / 2 随机
    cookie: String,
    level: String,
}

struct DspShared {
    eq: EqChain,
    surround: SurroundChain,
    fade: Fade,
}

pub struct AudioEngine {
    ctrl: Arc<Mutex<Control>>,
    dsp: Arc<Mutex<DspShared>>,
    buf: Arc<Mutex<VecDeque<f32>>>,
    gen: Arc<AtomicU64>,
    handle: Arc<Mutex<Option<AppHandle>>>,
}

impl AudioEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ctrl: Arc::new(Mutex::new(Control { volume: 1.0, level: "higher".into(), ..Default::default() })),
            dsp: Arc::new(Mutex::new(DspShared { eq: EqChain::default(), surround: SurroundChain::default(), fade: Fade::default() })),
            buf: Arc::new(Mutex::new(VecDeque::with_capacity(CAP))),
            gen: Arc::new(AtomicU64::new(0)),
            handle: Arc::new(Mutex::new(None)),
        })
    }

    pub fn start(self: &Arc<Self>, handle: AppHandle) {
        *self.handle.lock().unwrap() = Some(handle.clone());
        self.spawn_output();
        self.spawn_decoder();
        info!(target: "gomusic::audio", "engine started (ffmpeg-or-symphonia + cpal + dsp)");
    }

    fn emit(&self, c: &Control) {
        if let Some(h) = self.handle.lock().unwrap().as_ref() {
            let _ = h.emit("player://state", json!({
                "playing": c.playing,
                "positionMs": c.position_ms,
                "durationMs": c.duration_ms,
                "volume": c.volume,
                "song": c.queue.get(c.index).cloned().unwrap_or_default(),
            }));
        }
    }

    fn spawn_output(self: &Arc<Self>) {
        let dsp = self.dsp.clone();
        let buf = self.buf.clone();
        let ctrl = self.ctrl.clone();
        std::thread::spawn(move || {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => { warn!(target: "gomusic::audio", "no output device"); return }
            };
            let cfg = cpal::StreamConfig { channels: 2, sample_rate: cpal::SampleRate(SR), buffer_size: cpal::BufferSize::Default };
            let supported = device.supported_output_configs().ok().map(|mut v| v.any(|c| {
                c.channels() >= 2 && c.min_sample_rate().0 <= SR && c.max_sample_rate().0 >= SR
            })).unwrap_or(true);
            let final_cfg = if supported { cfg } else {
                match device.default_output_config() { Ok(c) => c.config(), Err(_) => return }
            };
            let channels = final_cfg.channels.max(2) as usize;
            let err_fn = |e| warn!(target: "gomusic::audio", "cpal: {}", e);
            let cb_dsp = dsp.clone();
            let cb_buf = buf.clone();
            let cb_ctrl = ctrl.clone();
            let stream = device.build_output_stream(
                &final_cfg,
                move |data: &mut [f32], _| {
                    let playing = cb_ctrl.lock().map(|c| c.playing).unwrap_or(false);
                    let volume = cb_ctrl.lock().map(|c| c.volume).unwrap_or(1.0);
                    let mut d = cb_dsp.lock().unwrap();
                    let mut b = cb_buf.lock().unwrap();
                    for frame in data.chunks_mut(channels) {
                        let (l, r) = if playing && b.len() >= 2 {
                            (b.pop_front().unwrap_or(0.0), b.pop_front().unwrap_or(0.0))
                        } else { (0.0, 0.0) };
                        let (l, r) = d.eq.process(l, r);
                        let (l, r) = d.surround.process(l, r);
                        let g = d.fade.gain() * volume;
                        let (l, r) = (soft_limit(l * g), soft_limit(r * g));
                        frame[0] = l;
                        if channels > 1 { frame[1] = r }
                        for x in frame.iter_mut().skip(2) { *x = 0.0 }
                    }
                },
                err_fn,
                None,
            );
            match stream {
                Ok(s) => {
                    if let Err(e) = s.play() { warn!(target: "gomusic::audio", "play: {}", e); return }
                    // Stream 必须留在本线程栈上保持存活；窗口最小化仍继续输出。
                    loop { std::thread::park() }
                }
                Err(e) => warn!(target: "gomusic::audio", "build_output_stream: {}", e),
            }
        });
    }

    fn spawn_decoder(self: &Arc<Self>) {
        let ctrl = self.ctrl.clone();
        let buf = self.buf.clone();
        let dsp = self.dsp.clone();
        let gen = self.gen.clone();
        let handle_holder = self.handle.clone();
        let emit_arc: Arc<dyn Fn(&Control) + Send + Sync> = Arc::new(move |c: &Control| {
            // 通过全局事件总线发射（与 spawn_output 同一 AppHandle，由 main.rs 注册）
            if let Some(h) = handle_holder.lock().unwrap().as_ref() {
                let _ = h.emit("player://state", json!({
                    "playing": c.playing,
                    "positionMs": c.position_ms,
                    "durationMs": c.duration_ms,
                    "volume": c.volume,
                    "song": c.queue.get(c.index).cloned().unwrap_or_default(),
                }));
            }
        });
        std::thread::spawn(move || {
            let mut last_gen = u64::MAX;
            loop {
                let g = gen.load(Ordering::Relaxed);
                let snap = ctrl.lock().map(|c| (
                    c.playing, c.queue.get(c.index).cloned(), c.seek_ms, c.uri_override.clone(),
                    c.cookie.clone(), c.level.clone(), c.repeat,
                )).unwrap_or((false, None, None, None, String::new(), "higher".into(), 0));
                let (playing, song, seek, override_uri, cookie, level, repeat) = snap;
                if !playing { std::thread::sleep(Duration::from_millis(20)); continue }
                if song.is_none() && override_uri.is_none() { std::thread::sleep(Duration::from_millis(50)); continue }
                if g == last_gen { std::thread::sleep(Duration::from_millis(20)); continue }
                last_gen = g;

                if let Ok(mut c) = ctrl.lock() {
                    c.seek_ms = None;
                    c.duration_ms = song.as_ref().map(|s| s.duration_ms).unwrap_or(0);
                }
                if let Ok(mut b) = buf.lock() { b.clear() }
                dsp.lock().unwrap().fade.to(1.0);

                let uri = override_uri.or_else(|| resolve_online_uri(song.as_ref(), &cookie, &level));
                let Some(uri) = uri else {
                    warn!(target: "gomusic::audio", "no playable uri");
                    continue;
                };
                let start_ms = seek.unwrap_or(0).max(0);
                if let Ok(mut c) = ctrl.lock() { c.position_ms = start_ms; emit_arc(&c) }

                let (total_frames, ok) = decode_into(&uri, start_ms, &buf, &gen, &g);
                if !ok { continue }
                let end_ms = start_ms + (total_frames as i64 / 48);
                if let Ok(mut c) = ctrl.lock() { c.position_ms = end_ms }

                // STATE_ENDED → 队列下一首（单曲循环 / 队列仅一首按原逻辑不跳）
                if gen.load(Ordering::Relaxed) != g { continue }
                let mut c = ctrl.lock().unwrap();
                if c.playing {
                    let n = c.queue.len();
                    if n > 0 {
                        if repeat == 1 { /* 单曲循环：不推进 index */ }
                        else if repeat == 2 { c.index = (c.index * 7 + 3) % n }
                        else { c.index = (c.index + 1) % n }
                    }
                    c.position_ms = 0;
                    c.seek_ms = None;
                    c.uri_override = None;
                    emit_arc(&c);
                }
                gen.fetch_add(1, Ordering::Relaxed);
            }
        });
    }

    // ── 对外控制 ────────────────────────────────────────────
    fn bump(&self) { self.gen.fetch_add(1, Ordering::Relaxed); }

    pub fn play(&self, uri: Option<String>) {
        let mut c = self.ctrl.lock().unwrap();
        if let Some(u) = uri { c.uri_override = Some(u) }
        c.playing = true;
        self.emit(&c);
        drop(c);
        self.dsp.lock().unwrap().fade.to(1.0);
        self.bump();
    }
    pub fn pause(&self) {
        let mut c = self.ctrl.lock().unwrap();
        c.playing = false;
        self.emit(&c);
        drop(c);
        self.dsp.lock().unwrap().fade.to(0.0);
    }
    pub fn seek(&self, position_ms: i64) {
        let mut c = self.ctrl.lock().unwrap();
        c.seek_ms = Some(position_ms.max(0));
        c.position_ms = position_ms.max(0);
        self.emit(&c);
        drop(c);
        if let Ok(mut b) = self.buf.lock() { b.clear() }
        self.bump();
    }
    pub fn next(&self) { self.step(1) }
    pub fn prev(&self) { self.step(-1) }
    fn step(&self, delta: isize) {
        let mut c = self.ctrl.lock().unwrap();
        let n = c.queue.len();
        if n > 0 { c.index = ((c.index as isize + delta).rem_euclid(n as isize)) as usize }
        c.position_ms = 0; c.seek_ms = None; c.uri_override = None;
        self.emit(&c);
        drop(c);
        self.bump();
    }
    pub fn set_queue(&self, songs: Vec<Song>) {
        let mut c = self.ctrl.lock().unwrap();
        c.queue = songs; c.index = 0; c.position_ms = 0; c.uri_override = None;
        self.emit(&c);
        drop(c);
        self.bump();
    }
    pub fn set_volume(&self, v: f32) {
        let mut c = self.ctrl.lock().unwrap();
        c.volume = v.clamp(0.0, 1.0);
        self.emit(&c);
    }
    pub fn set_repeat(&self, r: u8) { self.ctrl.lock().unwrap().repeat = r.min(2) }
    pub fn set_cookie(&self, cookie: String) { self.ctrl.lock().unwrap().cookie = cookie }
    pub fn set_level(&self, level: String) { self.ctrl.lock().unwrap().level = level }
    pub fn apply_eq(&self, cfg: EqConfig) { self.dsp.lock().unwrap().eq.apply_config(&cfg); self.dsp.lock().unwrap().fade.to(1.0) }
    pub fn apply_surround(&self, cfg: SurroundConfig) { self.dsp.lock().unwrap().surround.apply_config(&cfg); self.dsp.lock().unwrap().fade.to(1.0) }
    pub fn state(&self) -> serde_json::Value {
        let c = self.ctrl.lock().unwrap();
        json!({ "playing": c.playing, "positionMs": c.position_ms, "durationMs": c.duration_ms,
                "volume": c.volume, "queue": c.queue, "index": c.index, "repeat": c.repeat,
                "song": c.queue.get(c.index).cloned().unwrap_or_default() })
    }

    pub fn current_song(&self) -> Option<Song> {
        let c = self.ctrl.lock().unwrap();
        c.queue.get(c.index).cloned()
    }
}

/// 在线歌曲取地址：沿 NM_LEVELS 相邻回退，风控码立即终止，缓存记账用服务端返回档位。
fn resolve_online_uri(song: Option<&Song>, cookie: &str, level: &str) -> Option<String> {
    let song = song?;
    if let Some(p) = &song.local_path { return Some(p.clone()) }
    let levels = ["jymaster", "vivid", "dolby", "jyeffect", "hires", "lossless", "exhigh", "higher", "standard"];
    let start = levels.iter().position(|l| *l == level).unwrap_or(7);
    let client = util::http_client();
    for lv in &levels[start..] {
        let mut url = reqwest::Url::parse("https://mapi.myocd.de5.net/song/url/v1").ok()?;
        url.query_pairs_mut()
            .append_pair("id", &song.id.to_string())
            .append_pair("level", lv)
            .append_pair("unblock", "true")
            .append_pair("timestamp", &std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis().to_string())
            .append_pair("randomCNIP", "true");
        if !cookie.is_empty() { url.query_pairs_mut().append_pair("cookie", cookie); }
        let v: serde_json::Value = match client.get(url).send().and_then(|r| r.json()) { Ok(v) => v, Err(_) => continue };
        let code = v.get("code").and_then(serde_json::Value::as_i64).unwrap_or(200);
        if [-462, 10004, 460].contains(&code) { warn!(target: "gomusic::audio", "risk code {}", code); return None }
        if let Some(u) = v.get("data").and_then(|d| d.as_array()).and_then(|a| a.first())
            .and_then(|x| x.get("url")).and_then(|u| u.as_str()) {
            if !u.is_empty() { return Some(u.replace("http://", "https://")) }
        }
    }
    None
}

/// 录制阶段复用：把下载好的音频解成 48k 立体声 f32 PCM（不经过输出设备）。
pub fn decode_to_pcm(uri: &str) -> Option<(Vec<f32>, u32)> {
    if let Some(ff) = util::ffmpeg_bin() {
        if let Some(v) = decode_ffmpeg_all(&ff, uri) { if !v.is_empty() { return Some((v, SR)) } }
    }
    let buf = Arc::new(Mutex::new(VecDeque::with_capacity(CAP)));
    let gen = Arc::new(AtomicU64::new(0));
    let collector = Arc::new(Mutex::new(Vec::<f32>::new()));
    let (b2, c2, done) = (buf.clone(), collector.clone(), Arc::new(AtomicU64::new(0)));
    let d2 = done.clone();
    let drainer = std::thread::spawn(move || loop {
        {
            let mut b = b2.lock().unwrap();
            if !b.is_empty() {
                let mut c = c2.lock().unwrap();
                while let Some(s) = b.pop_front() { c.push(s) }
            }
        }
        if d2.load(Ordering::Relaxed) == 1 { break }
        std::thread::sleep(Duration::from_millis(1));
    });
    let (frames, ok) = decode_symphonia(uri, 0, &buf, &gen, &0);
    done.store(1, Ordering::Relaxed);
    let _ = drainer.join();
    if let Ok(mut b) = buf.lock() {
        let mut c = collector.lock().unwrap();
        while let Some(s) = b.pop_front() { c.push(s) }
    }
    let pcm = collector.lock().unwrap().clone();
    if pcm.is_empty() || (!ok && frames == 0) { None } else { Some((pcm, SR)) }
}

fn decode_ffmpeg_all(ff: &std::path::Path, uri: &str) -> Option<Vec<f32>> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(ff)
        .arg("-v").arg("error").arg("-nostdin")
        .arg("-i").arg(uri)
        .arg("-vn").arg("-ac").arg("2").arg("-ar").arg(SR.to_string())
        .arg("-f").arg("f32le").arg("-acodec").arg("pcm_f32le").arg("pipe:1")
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut all = Vec::new();
    child.stdout.take()?.read_to_end(&mut all).ok()?;
    let _ = child.wait();
    let mut pcm = Vec::with_capacity(all.len() / 4);
    for b in all.chunks_exact(4) { pcm.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
    Some(pcm)
}

/// 双路径解码：优先 ffmpeg sidecar / PATH（转成 48k 立体声 f32le），否则 symphonia 纯 Rust。
/// 返回 (解码帧数, 是否正常结束)。
fn decode_into(uri: &str, start_ms: i64, buf: &Arc<Mutex<VecDeque<f32>>>, gen: &Arc<AtomicU64>, g: &u64) -> (u64, bool) {
    if let Some(ff) = util::ffmpeg_bin() {
        if let Some(r) = decode_ffmpeg(&ff, uri, start_ms, buf, gen, g) { return r }
    }
    decode_symphonia(uri, start_ms, buf, gen, g)
}

fn push_pcm(buf: &Arc<Mutex<VecDeque<f32>>>, samples: &[f32], gen: &Arc<AtomicU64>, g: &u64) -> bool {
    if gen.load(Ordering::Relaxed) != *g { return false }
    loop {
        let len = buf.lock().map(|b| b.len()).unwrap_or(0);
        if len < CAP { break }
        if gen.load(Ordering::Relaxed) != *g { return false }
        std::thread::sleep(Duration::from_millis(2));
    }
    if let Ok(mut b) = buf.lock() { for s in samples { b.push_back(*s) } }
    true
}

fn decode_ffmpeg(ff: &std::path::Path, uri: &str, start_ms: i64, buf: &Arc<Mutex<VecDeque<f32>>>, gen: &Arc<AtomicU64>, g: &u64) -> Option<(u64, bool)> {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(ff);
    cmd.arg("-v").arg("error").arg("-nostdin");
    if start_ms > 0 { cmd.arg("-ss").arg(format!("{:.3}", start_ms as f64 / 1000.0)); }
    cmd.arg("-i").arg(uri)
        .arg("-vn").arg("-ac").arg("2").arg("-ar").arg(SR.to_string())
        .arg("-f").arg("f32le").arg("-acodec").arg("pcm_f32le").arg("pipe:1")
        .stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let mut out = child.stdout.take()?;
    let mut chunk = [0u8; 4096];
    let mut total = 0u64;
    loop {
        let n = out.read(&mut chunk).ok()?;
        if n == 0 { break }
        let mut pcm = Vec::with_capacity(n / 4);
        for b in chunk[..n].chunks_exact(4) { pcm.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
        total += pcm.len() as u64 / 2;
        if !push_pcm(buf, &pcm, gen, g) { let _ = child.kill(); return Some((total, false)) }
    }
    let _ = child.wait();
    Some((total, true))
}

fn decode_symphonia(uri: &str, start_ms: i64, buf: &Arc<Mutex<VecDeque<f32>>>, gen: &Arc<AtomicU64>, g: &u64) -> (u64, bool) {
    use symphonia::core::audio::{SampleBuffer, Signal};
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::default::{get_codecs, get_probe};
    use std::io::Cursor;

    let bytes: Vec<u8> = if uri.starts_with("http") {
        match util::http_client().get(uri).send() {
            Ok(r) => r.bytes().map(|b| b.to_vec()).unwrap_or_default(),
            Err(_) => return (0, false),
        }
    } else { std::fs::read(uri).unwrap_or_default() };
    if bytes.is_empty() { return (0, false) }
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = uri.split('?').next().and_then(|p| p.rsplit('.').next()) { hint.with_extension(ext); }
    let probed = match get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()) {
        Ok(p) => p, Err(e) => { warn!(target: "gomusic::audio", "probe: {}", e); return (0, false) }
    };
    let mut format = probed.format;
    let track = match format.default_track() { Some(t) => t.clone(), None => return (0, false) };
    let mut decoder = match get_codecs().make(&track.codec_params, &DecoderOptions::default()) {
        Ok(d) => d, Err(e) => { warn!(target: "gomusic::audio", "codec: {}", e); return (0, false) }
    };
    let track_id = track.id;
    let mut total = 0u64;
    let mut skip = (start_ms.max(0) as f64 / 1000.0) * SR as f64;
    while let Ok(packet) = format.next_packet() {
        if gen.load(Ordering::Relaxed) != *g { return (total, false) }
        if packet.track_id() != track_id { continue }
        let decoded = match decoder.decode(&packet) { Ok(d) => d, Err(_) => continue };
        let spec = *decoded.spec();
        let frames = decoded.frames() as u64;
        let mut sb = SampleBuffer::<f32>::new(frames, spec);
        sb.copy_interleaved_ref(decoded);
        let src_rate = spec.rate.max(1) as f64;
        let ch = spec.channels.count().max(1);
        let mut pcm = Vec::with_capacity((frames as f64 * SR as f64 / src_rate) as usize * 2);
        let s = sb.samples();
        for i in 0..frames as usize {
            let l = s.get(i * ch).copied().unwrap_or(0.0);
            let r = s.get(i * ch + 1).copied().unwrap_or(l);
            if skip > 0.0 { skip -= 1.0; continue }
            // 线性重采样到 48k
            let pos = i as f64 * SR as f64 / src_rate;
            let prev = (pos - 1.0).max(0.0) as usize;
            let frac = (pos - prev as f64) as f32;
            let pl = s.get(prev * ch).copied().unwrap_or(l);
            let pr = s.get(prev * ch + 1).copied().unwrap_or(r);
            pcm.push(pl + (l - pl) * frac);
            pcm.push(pr + (r - pr) * frac);
            total += 1;
        }
        if !push_pcm(buf, &pcm, gen, g) { return (total, false) }
    }
    (total, true)
}
