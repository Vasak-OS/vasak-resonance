use reqwest::Client;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::structs::{LyricsLine, TrackLyricsPayload};

const LRCLIB_BASE_URL: &str = "https://lrclib.net";

static LYRICS_CACHE: OnceLock<Mutex<HashMap<String, TrackLyricsPayload>>> = OnceLock::new();
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

const NEW_LYRICS_CACHE_RELATIVE_PATH: &str = ".config/resonance/lyrics-cache.json";
const LEGACY_LYRICS_CACHE_RELATIVE_PATH: &str = ".config/vasak/lyrics-cache.json";

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LrcLibRecord {
    id: u64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricsQuery {
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration_seconds: u64,
}

fn cache() -> &'static Mutex<HashMap<String, TrackLyricsPayload>> {
    LYRICS_CACHE.get_or_init(|| Mutex::new(load_persistent_cache()))
}

fn lyrics_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").map(PathBuf::from).ok().or_else(dirs::home_dir)?;
    Some(home.join(NEW_LYRICS_CACHE_RELATIVE_PATH))
}

fn legacy_lyrics_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").map(PathBuf::from).ok().or_else(dirs::home_dir)?;
    Some(home.join(LEGACY_LYRICS_CACHE_RELATIVE_PATH))
}

fn load_persistent_cache() -> HashMap<String, TrackLyricsPayload> {
    let Some(new_path) = lyrics_cache_path() else {
        return HashMap::new();
    };

    if let Ok(content) = fs::read_to_string(&new_path) {
        return serde_json::from_str::<HashMap<String, TrackLyricsPayload>>(&content)
            .unwrap_or_default();
    }

    let Some(legacy_path) = legacy_lyrics_cache_path() else {
        return HashMap::new();
    };

    let content = match fs::read_to_string(&legacy_path) {
        Ok(content) => content,
        Err(_) => return HashMap::new(),
    };

    let legacy_cache =
        serde_json::from_str::<HashMap<String, TrackLyricsPayload>>(&content).unwrap_or_default();

    if !legacy_cache.is_empty() {
        persist_cache(&legacy_cache);
    }

    legacy_cache
}

fn persist_cache(cache_map: &HashMap<String, TrackLyricsPayload>) {
    let Some(path) = lyrics_cache_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[lyrics] no se pudo crear carpeta de caché local: {}", err);
            return;
        }
    }

    let content = match serde_json::to_string(cache_map) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[lyrics] no se pudo serializar caché local: {}", err);
            return;
        }
    };

    if let Err(err) = fs::write(path, content) {
        eprintln!("[lyrics] no se pudo guardar caché local: {}", err);
    }
}

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(6))
            .connect_timeout(Duration::from_secs(3))
            .user_agent("VasakResonance/0.1 (+https://github.com/vasak-group)")
            .build()
            .expect("failed to create reqwest client")
    })
}

fn normalize_for_query(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("unknown artist")
        || trimmed.eq_ignore_ascii_case("unknown album")
        || trimmed.eq_ignore_ascii_case("unknown title")
    {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn query_key(query: &LyricsQuery) -> String {
    format!(
        "{}|{}|{}|{}",
        query.track_name.to_lowercase(),
        query.artist_name.to_lowercase(),
        query.album_name.to_lowercase(),
        query.duration_seconds
    )
}

fn parse_timestamp_to_ms(token: &str) -> Option<u64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }

    let minutes = parts[0].parse::<u64>().ok()?;
    let sec_parts = parts[1].split('.').collect::<Vec<_>>();
    let seconds = sec_parts.first()?.parse::<u64>().ok()?;

    let fractional_ms = match sec_parts.get(1) {
        Some(frac) if !frac.is_empty() => {
            if frac.len() >= 3 {
                frac[0..3].parse::<u64>().ok()?
            } else if frac.len() == 2 {
                frac.parse::<u64>().ok()? * 10
            } else {
                frac.parse::<u64>().ok()? * 100
            }
        }
        _ => 0,
    };

    Some(minutes * 60_000 + seconds * 1_000 + fractional_ms)
}

pub fn parse_lrc_lines(lrc_text: &str) -> Vec<LyricsLine> {
    let mut lines = Vec::<LyricsLine>::new();

    for raw_line in lrc_text.lines() {
        let mut rest = raw_line.trim();
        if rest.is_empty() {
            continue;
        }

        let mut stamps = Vec::<u64>::new();
        loop {
            if !rest.starts_with('[') {
                break;
            }
            let Some(end) = rest.find(']') else {
                break;
            };
            let token = &rest[1..end];
            if let Some(ms) = parse_timestamp_to_ms(token) {
                stamps.push(ms);
            }
            rest = rest[end + 1..].trim_start();
        }

        if stamps.is_empty() {
            continue;
        }

        let content = rest.to_string();
        for ms in stamps {
            lines.push(LyricsLine {
                time_ms: ms,
                text: content.clone(),
            });
        }
    }

    lines.sort_by(|a, b| match a.time_ms.cmp(&b.time_ms) {
        Ordering::Equal => a.text.cmp(&b.text),
        ordering => ordering,
    });
    lines
}

fn record_to_payload(source: &str, record: &LrcLibRecord) -> TrackLyricsPayload {
    let synced_lyrics = record
        .synced_lyrics
        .as_ref()
        .map(|lyrics| lyrics.trim().to_string())
        .filter(|lyrics| !lyrics.is_empty());
    let plain_lyrics = record
        .plain_lyrics
        .as_ref()
        .map(|lyrics| lyrics.trim().to_string())
        .filter(|lyrics| !lyrics.is_empty());

    let lines = synced_lyrics
        .as_ref()
        .map(|lrc| parse_lrc_lines(lrc))
        .unwrap_or_default();

    TrackLyricsPayload {
        source: format!(
            "{}:{}:{}-{}:{}",
            source, record.id, record.artist_name, record.track_name, record.album_name
        ),
        synced: !lines.is_empty(),
        instrumental: record.instrumental,
        plain_lyrics,
        synced_lyrics,
        lines,
    }
}

async fn fetch_signature_record(
    client: &Client,
    endpoint: &str,
    query: &LyricsQuery,
) -> Option<LrcLibRecord> {
    let duration = query.duration_seconds;
    let url = format!("{}/{}", LRCLIB_BASE_URL, endpoint);

    let response = match client
        .get(url.clone())
        .query(&[
            ("track_name", &query.track_name),
            ("artist_name", &query.artist_name),
            ("album_name", &query.album_name),
            ("duration", &duration.to_string()),
        ])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("[lyrics] request error for {}: {}", url, err);
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        eprintln!("[lyrics] non-success {} {} -> {}", url, status, body);
        return None;
    }

    match response.json::<LrcLibRecord>().await {
        Ok(rec) => Some(rec),
        Err(err) => {
            eprintln!("[lyrics] failed to parse JSON from {}: {}", url, err);
            None
        }
    }
}

async fn fetch_search_record(client: &Client, query: &LyricsQuery) -> Option<LrcLibRecord> {
    let base = format!("{} {}", query.artist_name, query.track_name)
        .trim()
        .to_string();
    if base.is_empty() {
        return None;
    }
    let url = format!("{}/api/search", LRCLIB_BASE_URL);

    let response = match client
        .get(url.clone())
        .query(&[("q", base.as_str())])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("[lyrics] search request error for {}: {}", url, err);
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        eprintln!("[lyrics] search non-success {} {} -> {}", url, status, body);
        return None;
    }

    let mut items = match response.json::<Vec<LrcLibRecord>>().await {
        Ok(list) => list,
        Err(err) => {
            eprintln!("[lyrics] failed to parse search JSON: {}", err);
            return None;
        }
    };

    if items.is_empty() {
        eprintln!("[lyrics] search returned empty list for q='{}'", base);
        return None;
    }

    items.sort_by(|a, b| {
        let da = (a.duration.round() as i64 - query.duration_seconds as i64).abs();
        let db = (b.duration.round() as i64 - query.duration_seconds as i64).abs();
        da.cmp(&db)
    });

    items.into_iter().next()
}

pub async fn fetch_track_lyrics(query: LyricsQuery) -> Result<TrackLyricsPayload, String> {
    let normalized = LyricsQuery {
        track_name: normalize_for_query(&query.track_name),
        artist_name: normalize_for_query(&query.artist_name),
        album_name: normalize_for_query(&query.album_name),
        duration_seconds: query.duration_seconds,
    };

    let key = query_key(&normalized);
    if let Ok(locked) = cache().lock() {
        if let Some(cached) = locked.get(&key) {
            return Ok(cached.clone());
        }
    }

    let client = http_client();

    let mut record = fetch_signature_record(client, "api/get", &normalized).await;
    if record.is_none() {
        record = fetch_search_record(client, &normalized).await;
    }

    let payload = match record {
        Some(record) => {
            if record.synced_lyrics.as_deref().unwrap_or("").is_empty()
                && record.plain_lyrics.as_deref().unwrap_or("").is_empty()
            {
                return Err("No se encontraron letras para esta canción".to_string());
            }

            record_to_payload("lrclib", &record)
        }
        None => return Err("No se encontraron letras para esta canción".to_string()),
    };

    if let Ok(mut locked) = cache().lock() {
        locked.insert(key, payload.clone());
        persist_cache(&locked);
    }

    Ok(payload)
}
