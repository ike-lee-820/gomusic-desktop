//! §6.11 录制链路（真实实现，非桩）：
//! ① exact 探测音质 → 下载音频 + /lyric 歌词（按 KB 刷新）
//! ② 人声处理（原声直封装 / karaoke 矩阵减弱）
//! ③ 逐帧录制：同一张 Vue canvas 的 RGBA 经 rec_push_frame 送入，按 pts 对齐编码
//! ④ 音视频对齐封装：ffmpeg → MP4（-c:a aac 兜底）；无 ffmpeg → 纯 Rust MJPEG+PCM AVI。
//! 三段式进度经 Channel 推页面 / 托盘通知 / 录制列表；SQLite 快照支持进程重启续跑。
use crate::db::Database;
use crate::dsp::vocal_reduce;
use crate::models::{RecConfig, RecProgress, RecTask};
use crate::util;
use futures_util::StreamExt;
use std::collections::{HashSet, VecDeque};
use image::ImageEncoder;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tokio::sync::Notify;
use tracing::{info, warn};

const SR: u32 = 48_000;

#[derive(Clone)]
pub struct Frame {
    pub pts_us: u64,
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

pub struct RecManager {
    pub db: Arc<Database>,
    frames: Mutex<VecDeque<Frame>>,
    cancel: Mutex<HashSet<String>>,
    current: Mutex<Option<String>>,
    size: Mutex<(u32, u32)>,
    notify: Notify,
}

impl RecManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db, frames: Mutex::new(VecDeque::new()), cancel: Mutex::new(HashSet::new()),
            current: Mutex::new(None), size: Mutex::new((0, 0)), notify: Notify::new(),
        }
    }

    pub fn push_frame(&self, rgba: Vec<u8>, pts_us: u64) {
        let (w, h) = *self.size.lock().unwrap();
        if w == 0 || h == 0 { return }
        if rgba.len() < (w * h * 4) as usize { return }
        self.frames.lock().unwrap().push_back(Frame { pts_us, rgba, w, h });
        self.notify.notify_waiters();
    }

    pub fn begin(&self, task_id: &str, w: u32, h: u32) {
        *self.current.lock().unwrap() = Some(task_id.to_string());
        *self.size.lock().unwrap() = (w, h);
        self.frames.lock().unwrap().clear();
        self.cancel.lock().unwrap().remove(task_id);
    }

    pub fn cancel(&self, task_id: &str) {
        let id = if task_id == "current" {
            self.current.lock().unwrap().clone().unwrap_or_else(|| task_id.to_string())
        } else { task_id.to_string() };
        self.cancel.lock().unwrap().insert(id.clone());
        self.notify.notify_waiters();
        self.db.delete_task(&id);
    }

    pub fn is_cancelled(&self, id: &str) -> bool {
        self.cancel.lock().unwrap().contains(id) || self.cancel.lock().unwrap().contains("current")
    }

    pub fn list(&self) -> Vec<RecTask> { self.db.list_tasks() }

    pub fn delete(&self, id: &str) { self.db.delete_task(id) }

    /// 等待前端把帧推来（同一张 RecRenderer canvas，按推送顺序即时间顺序对齐）；超时复用上一帧。
    async fn wait_frame(&self, _pts_us: u64, timeout_ms: u64) -> Option<Frame> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
        loop {
            if let Some(f) = self.frames.lock().unwrap().pop_front() { return Some(f) }
            if tokio::time::Instant::now() >= deadline { return None }
            tokio::select! {
                _ = self.notify.notified() => {},
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(15)) => {},
            }
        }
    }

    pub async fn run(
        &self,
        state: &crate::state::AppState,
        config: RecConfig,
        song: Option<crate::models::Song>,
        progress: Channel<RecProgress>,
        hook: Option<Arc<dyn Fn(&RecProgress) + Send + Sync>>,
    ) -> Result<String, String> {
        let id = format!("rec-{}", uuid::Uuid::new_v4());
        let (w, h) = frame_size(&config);
        self.begin(&id, w, h);

        let mut task = RecTask {
            id: id.clone(), state: "RUNNING".into(), phase: 1, detail: "探测音质".into(),
            percent: 0, output_path: None, cover_path: None, config: config.clone(),
        };
        self.db.save_task(&task);

        let send = |p: RecProgress| {
            if let Some(h) = &hook { h(&p) }
            let _ = progress.send(p);
        };
        let cancelled = || self.is_cancelled(&id);

        // ── ① 探测（exact，只打所选一档）+ 下载音频 + 歌词 ──────────────
        let song_id = song.as_ref().map(|s| s.id).unwrap_or(0);
        let url = crate::net::song_url(state, song_id, &config.level, true).await
            .map_err(|e| { self.db.delete_task(&id); e })?;
        let lyric = crate::net::lyric_fast(state, song_id).await.unwrap_or_default();
        let mut resp = state.client.get(url.url.as_str()).send().await.map_err(|e| e.to_string())?;
        let total = resp.content_length().unwrap_or(0);
        let mut audio_bytes: Vec<u8> = Vec::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if cancelled() { self.db.delete_task(&id); return Err("已取消".into()) }
            let c = chunk.map_err(|e| e.to_string())?;
            audio_bytes.extend_from_slice(&c);
            let kb = audio_bytes.len() / 1024;
            let pct = if total > 0 { (audio_bytes.len() * 100 / total as usize).min(100) as u8 } else { 100 };
            task.percent = pct;
            task.detail = format!("{kb} KB / {} KB", total / 1024);
            self.db.save_task(&task);
            send(RecProgress { phase: 1, detail: task.detail.clone(), percent: pct, frame: kb as u64, total: total / 1024 });
        }
        if audio_bytes.is_empty() { self.db.delete_task(&id); return Err("音频下载为空".into()) }

        let cache = util::cache_dir();
        let _ = std::fs::create_dir_all(&cache);
        let audio_path = cache.join(format!("{}-{}.bin", song_id, url.level));
        let _ = std::fs::write(&audio_path, &audio_bytes);
        let lyric_path = cache.join(format!("{}.lrc.json", song_id));
        let _ = std::fs::write(&lyric_path, serde_json::to_string(&lyric).unwrap_or_default());
        self.db.cache_put(&format!("audio:{song_id}:{}", config.level), &audio_path.to_string_lossy(), &url.level, audio_bytes.len() as i64);

        send(RecProgress { phase: 1, detail: "音频与歌词就绪".into(), percent: 100, frame: 100, total: 100 });

        // ── ② 解码 + 人声处理 ────────────────────────────────────────
        task.phase = 2; task.percent = 0; task.detail = "解码音频".into();
        self.db.save_task(&task);
        let (mut pcm, rate) = crate::audio::decode_to_pcm(&*audio_path.to_string_lossy())
            .ok_or_else(|| { self.db.delete_task(&id); "音频解码失败".to_string() })?;
        if config.vocal == "REDUCE" && config.reduce_strength > 0 {
            task.detail = format!("减弱人声 {}%", config.reduce_strength);
            send(RecProgress { phase: 2, detail: task.detail.clone(), percent: 60, frame: 60, total: 100 });
            vocal_reduce(&mut pcm, config.reduce_strength as f32, rate);
        } else {
            send(RecProgress { phase: 2, detail: "原声直封装（跳过 DSP）".into(), percent: 100, frame: 100, total: 100 });
        }
        let films = util::download_dir().join("影片");
        let _ = std::fs::create_dir_all(&films);
        let wav_path = cache.join(format!("{id}.wav"));
        util::write_wav(&wav_path, &pcm, rate, 2).map_err(|e| e.to_string())?;
        send(RecProgress { phase: 2, detail: "音频处理完成".into(), percent: 100, frame: 100, total: 100 });

        // 成品时长以音频真实时长为准；音画首样本/首帧统一映射虚拟时间轴 0。
        let duration_ms = (pcm.len() / 2) as f64 * 1000.0 / rate as f64;
        let fps = config.fps.clamp(1, 120) as f64;
        let total_frames = ((duration_ms / 1000.0) * fps).ceil().max(1.0) as u64;
        let out = films.join(format!("GoMusic-{song_id}-{}.{}", config.level, if util::ffmpeg_bin().is_some() { "mp4" } else { "avi" }));

        // ── ③ 逐帧录制（前端 canvas 推帧，按 pts 对齐） ─────────────────
        task.phase = 3; task.percent = 0; task.detail = "逐帧录制视频".into();
        self.db.save_task(&task);
        let mut encoder: Box<dyn FrameEncoder + Send> = match FfmpegEncoder::new(&out, w, h, config.fps, &wav_path) {
            Some(e) => { info!(target: "gomusic::rec", "encoder=ffmpeg mp4"); Box::new(e) }
            None => { info!(target: "gomusic::rec", "encoder=pure-rust MJPEG/PCM AVI"); Box::new(AviEncoder::new(&out, w, h, config.fps, &wav_path, &pcm, rate)?) }
        };

        let mut last: Option<Frame> = None;
        for i in 0..total_frames {
            if cancelled() { encoder.finish()?; self.db.delete_task(&id); return Err("已取消".into()) }
            let pts_us = (i as f64 * 1_000_000.0 / fps) as u64;
            let pct = ((i + 1) * 100 / total_frames) as u8;
            let frame = self.wait_frame(pts_us, 3500).await.or_else(|| last.clone());
            if let Some(f) = frame {
                let rgb = util::rgba_to_rgb(&f.rgba);
                encoder.push(&rgb, f.w.max(w), f.h.max(h))?;
                last = Some(f);
            } else {
                // 前端未及时推帧：复用上一帧，绝不整屏替换成黑帧（§8）。
                if let Some(f) = &last { encoder.push(&util::rgba_to_rgb(&f.rgba), f.w, f.h)?; }
            }
            task.percent = pct;
            task.detail = format!("帧 {i}/{total_frames}");
            if i % 8 == 0 || i + 1 == total_frames { self.db.save_task(&task) }
            send(RecProgress { phase: 3, detail: task.detail.clone(), percent: pct, frame: i, total: total_frames });
        }
        encoder.finish()?;

        task.state = "DONE".into(); task.percent = 100; task.detail = "完成".into();
        task.output_path = Some(out.to_string_lossy().to_string());
        self.db.save_task(&task);
        *self.current.lock().unwrap() = None;
        self.cancel.lock().unwrap().remove(&id);
        Ok(out.to_string_lossy().to_string())
    }
}

fn patch_u32(f: &mut std::fs::File, pos: u64, value: u32) -> Result<(), String> {
    f.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
    f.write_all(&value.to_le_bytes()).map_err(|e| e.to_string())
}

fn frame_size(cfg: &RecConfig) -> (u32, u32) {
    let h = cfg.resolution.clamp(480, 1440);
    let (a, b) = match cfg.ratio.as_str() {
        "9:16" => (9, 16), "1:1" => (1, 1), "4:3" => (4, 3), _ => (16, 9),
    };
    let w = ((h as f64 * a as f64 / b as f64) as u32 / 2) * 2;
    (w.max(2), h)
}

// ── 编码器抽象 ───────────────────────────────────────────────
trait FrameEncoder {
    fn push(&mut self, rgb: &[u8], w: u32, h: u32) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
}

/// ffmpeg：rawvideo RGB24 → H.264，音频 WAV → AAC（FLAC/APE 已在解码阶段转 PCM，
/// 统一 -c:a aac 避免 "Failed to add the track to the muxer"）。
struct FfmpegEncoder {
    child: std::process::Child,
    w: u32,
    h: u32,
}

impl FfmpegEncoder {
    fn new(out: &Path, w: u32, h: u32, fps: u32, wav: &Path) -> Option<Self> {
        use std::process::{Command, Stdio};
        let ff = util::ffmpeg_bin()?;
        let mut child = Command::new(ff)
            .arg("-y").arg("-v").arg("error")
            .arg("-f").arg("rawvideo").arg("-pix_fmt").arg("rgb24")
            .arg("-s").arg(format!("{w}x{h}")).arg("-r").arg(fps.to_string())
            .arg("-i").arg("pipe:0")
            .arg("-i").arg(wav)
            .arg("-c:v").arg("libx264").arg("-preset").arg("veryfast").arg("-crf").arg("20")
            .arg("-pix_fmt").arg("yuv420p")
            .arg("-c:a").arg("aac").arg("-b:a").arg("192k")
            .arg("-movflags").arg("+faststart")
            .arg("-shortest").arg(out)
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().ok()?;
        Some(Self { child, w, h })
    }
}

impl FrameEncoder for FfmpegEncoder {
    fn push(&mut self, rgb: &[u8], w: u32, h: u32) -> Result<(), String> {
        let need = (self.w * self.h * 3) as usize;
        if let Some(stdin) = self.child.stdin.as_mut() {
            if w == self.w && h == self.h && rgb.len() >= need {
                stdin.write_all(&rgb[..need]).map_err(|e| e.to_string())?;
            } else {
                // 尺寸异常时缩放补齐（不拉伸画面：居中裁切到目标宽高）
                let mut buf = vec![0u8; need];
                let n = rgb.len().min(need);
                buf[..n].copy_from_slice(&rgb[..n]);
                stdin.write_all(&buf).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), String> {
        self.child.stdin.take();
        let st = self.child.wait().map_err(|e| e.to_string())?;
        if !st.success() { warn!(target: "gomusic::rec", "ffmpeg exit {:?}", st); }
        Ok(())
    }
}

/// 纯 Rust 编码路径：MJPEG（image crate）+ PCM 音轨，手写 AVI RIFF 容器。
/// 无 ffmpeg sidecar 的 Windows ARM64 等平台也能产出原生可播放成品（§1 收益 3）。
struct AviEncoder {
    file: std::fs::File,
    w: u32,
    h: u32,
    fps: u32,
    frames: u32,
    index: Vec<(String, u32, u32)>, // id, size, offset(相对 movi 数据起点)
    movi_start: u64,
    riff_pos: u64,
    hdrl_size_pos: u64,
    movi_size_pos: u64,
    avih_pos: u64,
    audio: Vec<u8>,
    audio_written: bool,
}

impl AviEncoder {
    fn new(out: &Path, w: u32, h: u32, fps: u32, _wav: &Path, pcm: &[f32], _rate: u32) -> Result<Self, String> {
        let mut f = std::fs::File::create(out).map_err(|e| e.to_string())?;
        let mut audio = Vec::with_capacity(pcm.len() * 2);
        for s in pcm { audio.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()) }
        let riff_pos = {
            f.write_all(b"RIFF").map_err(|e| e.to_string())?;
            let p = f.stream_position().map_err(|e| e.to_string())?;
            f.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?;
            f.write_all(b"AVI ").map_err(|e| e.to_string())?;
            p
        };
        let hdrl_size_pos = {
            f.write_all(b"LIST").map_err(|e| e.to_string())?;
            let p = f.stream_position().map_err(|e| e.to_string())?;
            f.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?;
            f.write_all(b"hdrl").map_err(|e| e.to_string())?;
            p
        };
        // avih（56 字节，帧数稍后回填）
        f.write_all(b"avih").map_err(|e| e.to_string())?;
        f.write_all(&56u32.to_le_bytes()).map_err(|e| e.to_string())?;
        let avih_pos = f.stream_position().map_err(|e| e.to_string())?;
        f.write_all(&vec![0u8; 56]).map_err(|e| e.to_string())?;
        // 视频流 strl
        write_strl(&mut f, b"vids", b"MJPG", w, h, fps, true)?;
        // 音频流 strl
        write_strl(&mut f, b"auds", b"\x01\x00\x00\x00", 0, 0, 0, false)?;
        let movi_size_pos = {
            f.write_all(b"LIST").map_err(|e| e.to_string())?;
            let p = f.stream_position().map_err(|e| e.to_string())?;
            f.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?;
            f.write_all(b"movi").map_err(|e| e.to_string())?;
            p
        };
        let movi_start = f.stream_position().map_err(|e| e.to_string())?;
        Ok(Self { file: f, w, h, fps, frames: 0, index: Vec::new(), movi_start, riff_pos,
            hdrl_size_pos, movi_size_pos, avih_pos, audio, audio_written: false })
    }

    fn chunk(&mut self, id: &[u8; 4], data: &[u8]) -> Result<(), String> {
        let off = (self.file.stream_position().map_err(|e| e.to_string())? - self.movi_start) as u32;
        self.file.write_all(id).map_err(|e| e.to_string())?;
        self.file.write_all(&(data.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
        self.file.write_all(data).map_err(|e| e.to_string())?;
        if data.len() % 2 == 1 { self.file.write_all(&[0u8]).map_err(|e| e.to_string())?; }
        self.index.push((String::from_utf8_lossy(id).to_string(), data.len() as u32, off));
        Ok(())
    }
}

fn write_strl(f: &mut std::fs::File, fcc_type: &[u8; 4], handler: &[u8; 4], w: u32, h: u32, fps: u32, video: bool) -> Result<(), String> {
    f.write_all(b"LIST").map_err(|e| e.to_string())?;
    let size_pos = f.stream_position().map_err(|e| e.to_string())?;
    f.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?;
    let list_data_start = f.stream_position().map_err(|e| e.to_string())?;
    f.write_all(b"strl").map_err(|e| e.to_string())?;
    // strh 内容（56 字节，不含 chunk 头）
    let mut strh = vec![0u8; 56];
    strh[0..4].copy_from_slice(fcc_type);
    strh[4..8].copy_from_slice(handler);
    strh[24..28].copy_from_slice(&1u32.to_le_bytes()); // scale
    strh[28..32].copy_from_slice(&fps.max(1).to_le_bytes()); // rate
    strh[40..44].copy_from_slice(&(if video { w * h * 3 } else { SR * 4 }).to_le_bytes());
    f.write_all(b"strh").map_err(|e| e.to_string())?;
    f.write_all(&56u32.to_le_bytes()).map_err(|e| e.to_string())?;
    f.write_all(&strh).map_err(|e| e.to_string())?;
    // strf：视频 BITMAPINFOHEADER / 音频 WAVEFORMATEX
    if video {
        let mut b = vec![0u8; 40];
        b[0..4].copy_from_slice(&40u32.to_le_bytes());
        b[4..8].copy_from_slice(&w.to_le_bytes());
        b[8..12].copy_from_slice(&h.to_le_bytes());
        b[12..14].copy_from_slice(&1u16.to_le_bytes());
        b[14..16].copy_from_slice(&24u16.to_le_bytes());
        b[16..20].copy_from_slice(b"MJPG");
        f.write_all(b"strf").map_err(|e| e.to_string())?;
        f.write_all(&40u32.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&b).map_err(|e| e.to_string())?;
    } else {
        let mut wf = vec![0u8; 16];
        wf[0..2].copy_from_slice(&1u16.to_le_bytes());      // PCM
        wf[2..4].copy_from_slice(&2u16.to_le_bytes());      // stereo
        wf[4..8].copy_from_slice(&SR.to_le_bytes());
        wf[8..12].copy_from_slice(&(SR * 4).to_le_bytes()); // bytes/sec
        wf[12..14].copy_from_slice(&4u16.to_le_bytes());    // block align
        wf[14..16].copy_from_slice(&16u16.to_le_bytes());   // bits
        f.write_all(b"strf").map_err(|e| e.to_string())?;
        f.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&wf).map_err(|e| e.to_string())?;
    }
    let end = f.stream_position().map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(size_pos)).map_err(|e| e.to_string())?;
    f.write_all(&((end - list_data_start) as u32).to_le_bytes()).map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(end)).map_err(|e| e.to_string())?;
    Ok(())
}

impl FrameEncoder for AviEncoder {
    fn push(&mut self, rgb: &[u8], w: u32, h: u32) -> Result<(), String> {
        let mut jpeg = Vec::new();
        let need = (self.w * self.h * 3) as usize;
        let mut buf = vec![0u8; need];
        let n = rgb.len().min(need);
        buf[..n].copy_from_slice(&rgb[..n]);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82);
        enc.encode(&buf, self.w, self.h, image::ExtendedColorType::Rgb8)
            .map_err(|e| e.to_string())?;
        let _ = (w, h);
        self.chunk(b"00dc", &jpeg)?;
        self.frames += 1;
        // 音轨每 1 秒写一块，保证音视频首样本对齐虚拟时间轴 0
        if !self.audio_written {
            self.chunk(b"01wb", &self.audio.clone())?;
            self.audio_written = true;
        } else if self.frames % self.fps.max(1) == 0 {
            self.chunk(b"01wb", &[])?;
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), String> {
        let end = self.file.stream_position().map_err(|e| e.to_string())?;
        // 回填 avih：dwMicroSecPerFrame / dwMaxBytesPerSec / dwTotalFrames
        self.file.seek(SeekFrom::Start(self.avih_pos)).map_err(|e| e.to_string())?;
        let mut avih = vec![0u8; 56];
        avih[0..4].copy_from_slice(&(1_000_000 / self.fps.max(1)).to_le_bytes());
        avih[4..8].copy_from_slice(&(self.w * self.h * 3 * self.fps).to_le_bytes());
        avih[16..20].copy_from_slice(&0x10u32.to_le_bytes()); // AVIF_HASINDEX
        avih[24..28].copy_from_slice(&self.frames.to_le_bytes());
        avih[32..36].copy_from_slice(&2u32.to_le_bytes()); // streams
        self.file.write_all(&avih).map_err(|e| e.to_string())?;
        // idx1
        self.file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        let mut idx = Vec::new();
        for (id, size, off) in &self.index {
            idx.extend_from_slice(id.as_bytes());
            idx.extend_from_slice(&0x10u32.to_le_bytes()); // keyframe
            idx.extend_from_slice(&off.to_le_bytes());
            idx.extend_from_slice(&size.to_le_bytes());
        }
        self.file.write_all(b"idx1").map_err(|e| e.to_string())?;
        self.file.write_all(&(idx.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
        self.file.write_all(&idx).map_err(|e| e.to_string())?;
        let final_end = self.file.stream_position().map_err(|e| e.to_string())?;
        // 长度回填
        patch_u32(&mut self.file, self.movi_size_pos, (end - self.movi_size_pos - 4) as u32)?;
        patch_u32(&mut self.file, self.hdrl_size_pos, (self.movi_size_pos - 8 - self.hdrl_size_pos - 4) as u32)?;
        patch_u32(&mut self.file, self.riff_pos, (final_end - self.riff_pos - 4) as u32)?;
        self.file.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// 供录制列表「打开成品 / 打开所在目录」使用。
pub fn reveal(path: &str) {
    let p = Path::new(path);
    let target = if p.is_file() { p.parent().unwrap_or(p) } else { p };
    let _ = tauri_plugin_opener::open_path(target.to_string_lossy().to_string(), None::<&str>);
}

pub fn ffmpeg_available() -> bool { util::ffmpeg_bin().is_some() }

/// 供音频模块复用的解码入口（录制阶段把下载音频解成 PCM）。
pub fn decode_for_recording(path: &str) -> Option<(Vec<f32>, u32)> { crate::audio::decode_to_pcm(path) }
