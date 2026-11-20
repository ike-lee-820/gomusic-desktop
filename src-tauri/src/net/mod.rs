//! §4 / §4.5 / §4.7 通用请求封装、音质降档链、歌词免登录快速通道。
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use crate::models::{ArtistDesc, Lyric, LyricLine, SearchResult, Song, SongUrl, Topic, Section, UpdateDoc, OFFLINE_CHANGELOG};
use crate::state::AppState;

/// 完整档位表，相邻回退，不跳档。
pub const NM_LEVELS: [&str; 9] = [
    "jymaster", "vivid", "dolby", "jyeffect", "hires",
    "lossless", "exhigh", "higher", "standard",
];

/// 风控终止码：立即终止，不逐档重试。
pub const RISK_CODES: [i64; 3] = [-462, 10004, 460];

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

/// 字段名容错：description / desc / briefDesc / introduction / data / result 多形态兼容。
pub fn pick<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for k in keys { if let Some(x) = v.get(k) { if !x.is_null() { return Some(x) } } }
    None
}

fn as_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// 通用请求：强制追加 unblock / cookie / timestamp / randomCNIP；失败重试 2 次。
/// fast=true 用于 /lyric：不带 cookie、不追加上述参数、不重试。
pub async fn request(
    state: &AppState,
    path: &str,
    mut params: Vec<(String, String)>,
    fast: bool,
) -> Result<Value, String> {
    if !fast {
        params.push(("unblock".into(), "true".into()));
        params.push(("timestamp".into(), now_ms().to_string()));
        params.push(("randomCNIP".into(), "true".into()));
        let cookie = state.cookie_value();
        if !cookie.is_empty() { params.push(("cookie".into(), cookie)); }
    }
    let url = if path.starts_with("http") { path.to_string() } else { format!("{}{}", state.base_url, path) };
    let attempts = if fast { 1 } else { 3 };
    let mut last = String::from("request failed");
    for attempt in 0..attempts {
        match state.client.get(&url).query(&params).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body: Value = resp.json().await.map_err(|e| e.to_string())?;
                let code = body.get("code").and_then(Value::as_i64).unwrap_or(200);
                if code == 200 { return Ok(body) }
                if RISK_CODES.contains(&code) {
                    warn!(target: "gomusic::net", "risk code {} -> abort", code);
                    return Err(format!("RISK:{code}"));
                }
                last = format!("business code {code}");
                if fast { break }
            }
            Ok(resp) => last = format!("http {}", resp.status()),
            Err(e) => last = e.to_string(),
        }
        if attempt + 1 < attempts { tokio::time::sleep(std::time::Duration::from_millis(200)).await }
    }
    Err(last)
}

/// 解析 [mm:ss.xxx] 时间轴；无时间戳的纯文本按行均分补轴。
pub fn parse_lrc(raw: &str) -> Lyric {
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut plain: Vec<String> = Vec::new();
    for row in raw.lines() {
        let trimmed = row.trim();
        if trimmed.is_empty() { continue }
        let mut rest = trimmed;
        let mut time_ms: Option<i64> = None;
        if trimmed.starts_with('[') {
            if let Some(end) = trimmed.find(']') {
                let tag = &trimmed[1..end];
                let parts: Vec<&str> = tag.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(m), Ok(s)) = (parts[0].parse::<i64>(), parts[1].parse::<f64>()) {
                        time_ms = Some(m * 60_000 + (s * 1000.0) as i64);
                    }
                }
                rest = &trimmed[end + 1..];
            }
        }
        match time_ms {
            Some(t) => lines.push(LyricLine { time_ms: t, text: rest.trim().to_string(), trans: None, roma: None }),
            None => plain.push(rest.trim().to_string()),
        }
    }
    if !lines.is_empty() {
        lines.sort_by_key(|l| l.time_ms);
        return Lyric { lines, has_timeline: true }
    }
    let step = 3000;
    Lyric {
        lines: plain.into_iter().enumerate().map(|(i, text)| LyricLine {
            time_ms: (i as i64) * step, text, trans: None, roma: None,
        }).collect(),
        has_timeline: false,
    }
}

/// /lyric 免登录快速通道：主请求为空才补一次带 randomCNIP 的（仍不带 cookie）。
pub async fn lyric_fast(state: &AppState, id: i64) -> Result<Lyric, String> {
    let path = format!("/lyric?id={id}");
    let first = request(state, &path, vec![], true).await?;
    let raw = as_text(pick(&first, &["lrc"]).and_then(|l| l.get("lyric")));
    let body = if raw.trim().is_empty() {
        request(state, &format!("/lyric?id={id}&randomCNIP=true"), vec![], true).await?
    } else { first };
    let lrc = as_text(pick(&body, &["lrc"]).and_then(|l| l.get("lyric")));
    let mut lyric = parse_lrc(&lrc);
    let trans = as_text(pick(&body, &["tlyric"]).and_then(|l| l.get("lyric")));
    if !trans.is_empty() {
        let tl = parse_lrc(&trans);
        for (i, l) in lyric.lines.iter_mut().enumerate() {
            if let Some(t) = tl.lines.get(i) { l.trans = Some(t.text.clone()) }
        }
    }
    Ok(lyric)
}

fn song_from(v: &Value) -> Song {
    let artists = v.get("artists").and_then(Value::as_array);
    Song {
        id: v.get("id").and_then(Value::as_i64).unwrap_or_default(),
        name: as_text(v.get("name")),
        artist: artists.and_then(|a| a.first()).map(|a| as_text(a.get("name"))).unwrap_or_default(),
        artist_id: artists.and_then(|a| a.first()).and_then(|a| a.get("id")).and_then(Value::as_i64),
        album: as_text(v.get("album").and_then(|a| a.get("name"))),
        album_id: v.get("album").and_then(|a| a.get("id")).and_then(Value::as_i64),
        pic_url: v.get("album").and_then(|a| a.get("picUrl")).and_then(Value::as_str).map(str::to_string),
        duration_ms: v.get("duration").and_then(Value::as_i64).unwrap_or_default(),
        source: "NM".into(),
        local_path: None,
        level: None,
    }
}

pub async fn search(state: &AppState, keywords: &str, type_: i32, limit: i64, offset: i64) -> Result<SearchResult, String> {
    let body = request(state, "/search", vec![
        ("keywords".into(), keywords.to_string()),
        ("type".into(), type_.to_string()),
        ("limit".into(), limit.to_string()),
        ("offset".into(), offset.to_string()),
    ], false).await?;
    let result = pick(&body, &["result", "data"]).cloned().unwrap_or(Value::Null);
    let songs = result.get("songs").and_then(Value::as_array)
        .map(|a| a.iter().map(song_from).collect()).unwrap_or_default();
    let total = pick(&result, &["songCount", "albumCount", "artistCount", "total"])
        .and_then(Value::as_i64).unwrap_or(0);
    let has_more = result.get("hasMore").and_then(Value::as_bool).unwrap_or(offset + limit < total);
    Ok(SearchResult { songs, total, has_more })
}

fn song_url_from(v: &Value, requested: &str) -> Option<SongUrl> {
    let url = v.get("url").and_then(Value::as_str).unwrap_or("");
    if url.is_empty() { return None }
    Some(SongUrl {
        url: url.to_string(),
        level: v.get("level").and_then(Value::as_str).unwrap_or(requested).to_string(),
        br: v.get("br").and_then(Value::as_i64).unwrap_or_default(),
        size: v.get("size").and_then(Value::as_i64).unwrap_or_default(),
        md5: v.get("md5").and_then(Value::as_str).unwrap_or_default().to_string(),
    })
}

/// 音质取地址：exact=true 只打一档且要求服务端返回档位一致；否则沿 NM_LEVELS 相邻回退。
pub async fn song_url(state: &AppState, id: i64, level: &str, exact: bool) -> Result<SongUrl, String> {
    let start = NM_LEVELS.iter().position(|l| *l == level).unwrap_or(NM_LEVELS.len() - 1);
    let chain: Vec<&str> = if exact { vec![NM_LEVELS[start]] } else { NM_LEVELS[start..].to_vec() };
    let mut last = String::from("所选档位不可用，请换档");
    for lv in chain {
        let body = match request(state, "/song/url/v1", vec![
            ("id".into(), id.to_string()), ("level".into(), lv.to_string()),
        ], false).await {
            Ok(v) => v,
            Err(e) => {
                if e.starts_with("RISK:") { return Err(e) }
                last = e; continue
            }
        };
        let item = body.get("data").and_then(Value::as_array).and_then(|a| a.first());
        if let Some(u) = item.and_then(|v| song_url_from(v, lv)) {
            if exact && u.level != lv {
                return Err(format!("所选档位 {} 不可用（返回 {}），请换档", lv, u.level))
            }
            info!(target: "gomusic::net", "song {} level {} -> {}", id, lv, u.level);
            return Ok(u)
        }
        last = format!("档位 {} 无可用地址", lv);
    }
    Err(last)
}

pub async fn probe_levels(state: &AppState, id: i64) -> Vec<String> {
    let mut out = Vec::new();
    for lv in NM_LEVELS {
        if song_url(state, id, lv, true).await.is_ok() { out.push(lv.to_string()) }
    }
    out
}

pub async fn artist_detail(state: &AppState, id: i64) -> Result<ArtistDesc, String> {
    let body = request(state, "/artist/desc", vec![("id".into(), id.to_string())], false).await?;
    let sections = body.get("introduction").and_then(Value::as_array).map(|a| a.iter().map(|s| Section {
        ti: as_text(s.get("ti")), txt: as_text(s.get("txt")),
    }).collect()).unwrap_or_default();
    let topics = body.get("topicData").and_then(Value::as_array).map(|a| a.iter().map(|t| Topic {
        title: as_text(pick(t, &["mainTitle", "title"])),
        summary: as_text(t.get("summary")),
        tags: t.get("tags").and_then(Value::as_array).map(|x| x.iter().map(|v| as_text(Some(v))).collect()).unwrap_or_default(),
    }).collect()).unwrap_or_default();
    Ok(ArtistDesc {
        brief_desc: as_text(pick(&body, &["briefDesc", "description", "desc", "introduction"])),
        sections, topics,
    })
}

pub async fn ugc_desc(state: &AppState, path: &str, id: i64) -> Result<ArtistDesc, String> {
    let body = request(state, &format!("{path}?id={id}"), vec![], false).await?;
    let data = pick(&body, &["data", "result"]).cloned().unwrap_or(body.clone());
    Ok(ArtistDesc {
        brief_desc: as_text(pick(&data, &["description", "desc", "briefDesc", "introduction"])),
        sections: Vec::new(), topics: Vec::new(),
    })
}

pub async fn translate(state: &AppState, text: &str, from: &str, to: &str) -> Result<String, String> {
    let body: Value = state.client
        .post("https://translate.mapi.myocd.de5.net/translate")
        .json(&serde_json::json!({ "text": text, "from": from, "to": to }))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    Ok(as_text(pick(&body, &["translation", "data", "result"])))
}

// ── 更新检查（§4.4）：默认 gh-proxy 加速，403/WAF/非 JSON 自动回退直连 ──────

/// 桌面端独立仓库，不复用手机端 tag（§10）。
pub const GITHUB_REPO: &str = "ike-lee-820/gomusic-desktop";

fn client_ua() -> String { format!("GoMusicDesktop/{}", env!("CARGO_PKG_VERSION")) }

/// 三个候选并发取；Cloudflare WAF 的 403 是 HTML，JSON 解析失败即丢弃，不把英文堆栈抛给界面。
async fn release_candidates(client: &reqwest::Client, url: &str, source: &str) -> Option<Vec<Value>> {
    let resp = match client
        .get(url)
        .header("User-Agent", client_ua())
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            warn!(target: "gomusic::update", "{} -> http {} (回退直连)", source, r.status());
            return None
        }
        Err(e) => {
            warn!(target: "gomusic::update", "{} -> {} (回退直连)", source, e);
            return None
        }
    };
    let text = resp.text().await.ok()?;
    let parsed: Value = serde_json::from_str(&text).ok()?;
    parsed.as_array().cloned()
}

fn looks_like_markdown(s: &str) -> bool {
    s.contains('#') || s.contains("- ") || s.contains('新增') || s.contains('修复')
}

/// yy.mm.次数 数值比较，不按字符串字典序。
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(s: &str) -> [i64; 3] {
        let mut out = [0i64; 3];
        for (i, p) in s.trim_start_matches('v').split('.').take(3).enumerate() {
            out[i] = p.parse().unwrap_or(0);
        }
        out
    }
    parts(a).cmp(&parts(b))
}

pub async fn check_update(state: &AppState, force: bool) -> Result<UpdateDoc, String> {
    let api = format!("https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=20");
    let proxy = format!("https://gh-proxy.com/{api}");
    let proxy_v6 = format!("https://v6.gh-proxy.com/{api}");
    info!(target: "gomusic::update", "check force={} via gh-proxy + v6 + direct", force);

    let (p, v6, d) = tokio::join!(
        release_candidates(&state.client, &proxy, "gh-proxy.com"),
        release_candidates(&state.client, &proxy_v6, "v6.gh-proxy.com"),
        release_candidates(&state.client, &api, "api.github.com"),
    );
    let candidates: Vec<(&str, Vec<Value>)> = [
        ("gh-proxy.com", p),
        ("v6.gh-proxy.com", v6),
        ("api.github.com", d),
    ]
    .into_iter()
    .filter_map(|(n, v)| v.map(|v| (n, v)))
    .collect();

    // 按首个 release 的 body 长度取最完整；短于最长 90% 或不像 Markdown 的不采用。
    let lens: Vec<usize> = candidates
        .iter()
        .map(|(_, v)| v.first().and_then(|r| r.get("body")).and_then(Value::as_str).unwrap_or("").len())
        .collect();
    let max_len = lens.iter().copied().max().unwrap_or(0);
    let mut best: Option<(&str, &Vec<Value>, usize)> = None;
    for (i, (name, list)) in candidates.iter().enumerate() {
        if max_len == 0 || lens[i] * 10 < max_len * 9 { continue }
        if let Some(body) = list.first().and_then(|r| r.get("body")).and_then(Value::as_str) {
            if !looks_like_markdown(body) { continue }
        }
        if best.map_or(true, |(_, _, l)| lens[i] > l) {
            best = Some((*name, list, lens[i]));
        }
    }

    if let Some((source, list, _)) = best {
        if let Some(r) = list.first() {
            let version = r.get("tag_name").and_then(Value::as_str).unwrap_or("")
                .trim_start_matches('v').to_string();
            let body = r.get("body").and_then(Value::as_str).unwrap_or("").to_string();
            let published_at = r.get("published_at").and_then(Value::as_str).map(str::to_string);
            let has_update = version_cmp(&version, env!("CARGO_PKG_VERSION")) == std::cmp::Ordering::Greater;
            let asset_url = r.get("assets").and_then(Value::as_array).and_then(|a| {
                a.iter().find(|x| {
                    x.get("name").and_then(Value::as_str).map(|n| {
                        let n = n.to_ascii_lowercase();
                        n.ends_with(".exe") || n.ends_with(".msi") || n.ends_with(".dmg")
                            || n.ends_with(".deb") || n.ends_with(".appimage") || n.ends_with(".rpm")
                    }).unwrap_or(false)
                }).or_else(|| a.first())
            })
            .and_then(|x| x.get("browser_download_url"))
            .and_then(Value::as_str)
            .map(str::to_string);
            // 下载默认走 gh-proxy.com/github.com 加速；同时保留直连兜底。
            let download_url = asset_url.as_ref().map(|u| format!("https://gh-proxy.com/{u}"));
            info!(target: "gomusic::update", "source={} version={} hasUpdate={}", source, version, has_update);
            return Ok(UpdateDoc {
                version, body, has_update, download_url,
                fallback_url: asset_url, published_at,
            })
        }
    }

    // 全部候选不可用（含 Cloudflare 403）：离线内置 CHANGELOG，界面照常可用。
    warn!(target: "gomusic::update", "all candidates unavailable -> offline changelog");
    Ok(UpdateDoc {
        version: env!("CARGO_PKG_VERSION").to_string(),
        body: OFFLINE_CHANGELOG.to_string(),
        ..Default::default()
    })
}
