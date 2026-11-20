//! §4.6 本地音乐（无网络请求）：walkdir + lofty + 自研标签读取。
//! 同名 .lrc 优先 → 内嵌同步歌词 → 内嵌文本（自动补时间轴）；UTF-8/UTF-16/GBK 自动识别。
//! 点封面/歌手不跳转、不触发在线取地址/音质探测/在线歌词。
use crate::models::{Lyric, Song};
use crate::net::parse_lrc;
use encoding_rs::{GBK, UTF_16LE, UTF_8};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const EXTS: &[&str] = &["mp3", "flac", "m4a", "aac", "ape", "wav", "ogg", "oga", "wma",
    "aiff", "aif", "dsf", "dff", "alac", "opus", "mp4"];

fn is_audio(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str())
        .map(|e| EXTS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
}

fn id_of(p: &Path) -> i64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    p.to_string_lossy().hash(&mut h);
    (h.finish() as i64).abs()
}

/// UTF-8 / UTF-16LE(BOM) / GBK 自动识别。
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        let (s, _, _) = UTF_16LE.decode(&bytes[2..]);
        return s.into_owned()
    }
    let (utf8, _, bad) = UTF_8.decode(bytes);
    if !bad { return utf8.into_owned() }
    let (gbk, _, _) = GBK.decode(bytes);
    gbk.into_owned()
}

/// 自研容器标签兜底：ID3v2 USLT/SYLT、APEv2 Lyrics、Vorbis LYRICS、MP4 ©lyr。
fn raw_embedded_lyrics(bytes: &[u8]) -> Option<String> {
    for marker in [b"USLT".as_slice(), b"SYLT".as_slice(), b"\xA9lyr".as_slice(), b"LYRICS".as_slice(), b"UNSYNCEDLYRICS".as_slice()] {
        let mut from = 0;
        while let Some(pos) = bytes[from..].windows(marker.len()).position(|w| w == marker) {
            let p = from + pos + marker.len();
            // 跳过 3 字节语言码 / 帧头，找可打印的连续文本
            let tail = &bytes[p..bytes.len().min(p + 8192)];
            let start = tail.iter().position(|b| *b == 0 || *b >= 0x20).unwrap_or(0);
            let text = decode_text(&tail[start..]);
            let clean: String = text.chars().filter(|c| {
                c.is_alphanumeric() || c.is_whitespace() || "，。！？、；：'‘’\"“”’—…·,.!?-()[]".contains(*c)
            }).collect();
            let clean = clean.trim().to_string();
            if clean.chars().count() >= 4 { return Some(clean) }
            from = p;
            if from >= bytes.len() { break }
        }
    }
    None
}

fn sibling_lrc(p: &Path) -> Option<String> {
    let lrc = p.with_extension("lrc");
    if lrc.is_file() {
        return std::fs::read(&lrc).ok().map(|b| decode_text(&b))
    }
    None
}

fn embedded_lyrics(p: &Path, bytes: &[u8]) -> Option<String> {
    // lofty 统一读 ItemKey::Lyrics（ID3v2 USLT / APEv2 / Vorbis / MP4 ©lyr 均映射到此键）
    if let Ok(tagged) = lofty::read_from_path(p) {
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        if let Some(t) = tag {
            if let Some(v) = t.get_string(&lofty::tag::ItemKey::Lyrics) {
                if !v.trim().is_empty() { return Some(v.to_string()) }
            }
        }
    }
    raw_embedded_lyrics(bytes)
}

pub fn scan(paths: &[String]) -> Vec<Song> {
    let roots: Vec<PathBuf> = if paths.is_empty() {
        [dirs::audio_dir(), dirs::download_dir(), Some(PathBuf::from("."))]
            .into_iter().flatten().collect()
    } else { paths.iter().map(PathBuf::from).collect() };
    let mut out = Vec::new();
    for root in roots {
        for e in WalkDir::new(&root).follow_links(false).max_depth(6).into_iter().filter_map(Result::ok) {
            if e.file_type().is_file() && is_audio(e.path()) { out.push(meta(e.path())) }
        }
    }
    out
}

pub fn meta(p: &Path) -> Song {
    let mut name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("未知歌曲").to_string();
    let (mut artist, mut album, mut duration_ms) = (String::new(), String::new(), 0i64);
    if let Ok(tagged) = lofty::read_from_path(p) {
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        if let Some(t) = tag {
            if let Some(v) = t.title() { if !v.is_empty() { name = v.to_string() } }
            if let Some(v) = t.artist() { artist = v.to_string() }
            if let Some(v) = t.album() { album = v.to_string() }
        }
        duration_ms = tagged.properties().duration().as_millis() as i64;
    }
    Song {
        id: id_of(p), name, artist, artist_id: None, album, album_id: None,
        pic_url: None, duration_ms, source: "LOCAL".into(),
        local_path: Some(p.to_string_lossy().to_string()), level: None,
    }
}

pub fn cover(p: &Path) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(p).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let pic = tag.pictures().first()?;
    Some(pic.data().to_vec())
}

/// 优先级：同名 .lrc → 内嵌同步歌词 → 内嵌文本（自动补时间轴）。
pub fn lyric(p: &Path) -> Lyric {
    let raw = sibling_lrc(p).or_else(|| {
        std::fs::read(p).ok().and_then(|b| embedded_lyrics(p, &b))
    }).unwrap_or_default();
    let mut l = parse_lrc(&raw);
    // SYLT 的 MPEG 帧时间按 26ms 近似换算：无时间轴文本按行均分已由 parse_lrc 完成。
    if !l.has_timeline && !l.lines.is_empty() {
        for (i, line) in l.lines.iter_mut().enumerate() { line.time_ms = (i as i64) * 26 * 40 }
    }
    l
}
