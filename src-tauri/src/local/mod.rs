//! §4.6 本地元数据：lofty + 自研标签读取，同名 .lrc 优先，内嵌 USLT/SYLT/©lyr。
//! 待接入依赖：lofty / walkdir / encoding_rs。点封面/歌手不跳转、不触发在线查询。
pub const STATUS: &str = "pending: lofty metadata + embedded lyrics";

pub trait LocalIndex {
    fn scan(&mut self, paths: &[String]) -> usize;
    fn read_lyric(&self, path: &str) -> Option<String>;
}
