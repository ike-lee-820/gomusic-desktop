//! §5 数据模型：serde camelCase，与 src/api/types.ts 手工镜像保持同步。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: i64,
    pub name: String,
    pub artist: String,
    pub artist_id: Option<i64>,
    pub album: String,
    pub album_id: Option<i64>,
    pub pic_url: Option<String>,
    pub duration_ms: i64,
    pub source: String,
    pub local_path: Option<String>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub time_ms: i64,
    pub text: String,
    pub trans: Option<String>,
    pub roma: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lyric {
    pub lines: Vec<LyricLine>,
    pub has_timeline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SongUrl {
    pub url: String,
    pub level: String,
    pub br: i64,
    pub size: i64,
    pub md5: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub songs: Vec<Song>,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub ti: String,
    pub txt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDesc {
    pub brief_desc: String,
    pub sections: Vec<Section>,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Band {
    pub freq: i32,
    pub gain: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqConfig {
    pub preset: String,
    pub strength: f32,
    pub bands: Vec<Band>,
    pub hpf: f32,
    pub lpf: f32,
    pub device_calib: String,
    pub ab_loudness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurroundConfig {
    pub enabled: bool,
    pub strength: f32,
    pub scene: String,
    pub width: f32,
    pub distance: f32,
    pub taps: Vec<u32>,
    pub reverb: f32,
    pub eco: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecConfig {
    pub ratio: String,
    pub resolution: u32,
    pub fps: u32,
    pub level: String,
    pub bitrate: u32,
    pub lyric_size: u8,
    pub info_size: u8,
    pub info_zone: f32,
    pub lyric_offset_ms: i32,
    pub custom_lrc: Option<String>,
    pub font_source: String,
    pub wght: u16,
    pub show_trans: bool,
    pub style: String,
    pub background: String,
    pub vocal: String,
    pub reduce_strength: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecTask {
    pub id: String,
    pub state: String,
    pub phase: u8,
    pub detail: String,
    pub percent: u8,
    pub output_path: Option<String>,
    pub cover_path: Option<String>,
    pub config: RecConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecProgress {
    pub phase: u8,
    pub detail: String,
    pub percent: u8,
    pub frame: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDoc {
    pub version: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoginState {
    pub logged: bool,
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}
