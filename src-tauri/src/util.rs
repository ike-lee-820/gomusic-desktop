//! 跨模块共享工具：应用数据目录、ffmpeg 探测、PCM WAV、RGBA→RGB。
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("GoMusic")
}

pub fn cache_dir() -> PathBuf { data_dir().join("cache") }

pub fn download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| data_dir()).join("GoMusic")
}

/// ffmpeg sidecar 优先（binaries/ 下按 triple 命名），否则回退系统 PATH。
/// 找不到时由 rec/ 走纯 Rust 编码路径，不影响产出成品。
pub fn ffmpeg_bin() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let triple = std::env::var("TAURI_ENV_TARGET_TRIPLE").unwrap_or_default();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if !triple.is_empty() {
        candidates.push(data_dir().join("binaries").join(format!("ffmpeg-{triple}{}", if cfg!(windows) { ".exe" } else { "" })));
    }
    candidates.push(data_dir().join("binaries").join(exe));
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(parent) = exe_dir.parent() {
            candidates.push(parent.join(exe));
            candidates.push(parent.join(format!("ffmpeg-{triple}{}", if cfg!(windows) { ".exe" } else { "" })));
        }
    }
    for p in candidates { if p.is_file() { return Some(p) } }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(if cfg!(windows) { ';' } else { ':' }) {
            let p = Path::new(dir).join(exe);
            if p.is_file() { return Some(p) }
        }
    }
    None
}

/// 写 16-bit PCM WAV（小端，显式声道/采样率）。
pub fn write_wav(path: &Path, pcm: &[f32], sample_rate: u32, channels: u16) -> std::io::Result<()> {
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    let data_len = (pcm.len() * 2) as u32;
    let bytes_per_sec = sample_rate * channels as u32 * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());          // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&bytes_per_sec.to_le_bytes());
    out.extend_from_slice(&(channels as u16 * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, out)
}

/// RGBA → RGB24（ffmpeg rawvideo / JPEG 编码共用）。
pub fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        let a = px[3] as f32 / 255.0;
        // 预乘到近黑底，保证透明像素不会变成白斑
        out.push((px[0] as f32 * a) as u8);
        out.push((px[1] as f32 * a) as u8);
        out.push((px[2] as f32 * a) as u8);
    }
    out
}

pub fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("GoMusicDesktop/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}
