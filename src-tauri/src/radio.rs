use serde::{Deserialize, Serialize};
use std::io::Read;

/// Represents a radio station from radio-browser.info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RadioStation {
    #[serde(rename = "stationuuid")]
    pub uuid: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub favicon: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub votes: Option<u32>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub bitrate: Option<u32>,
}

impl RadioStation {
    pub fn tag_list(&self) -> Vec<String> {
        self.tags
            .as_ref()
            .map(|t| t.split(',').map(|s| s.trim().to_lowercase()).collect())
            .unwrap_or_default()
    }
}

/// Fetches radio stations from radio-browser.info API
pub async fn fetch_stations(tags: Vec<&str>) -> Result<Vec<RadioStation>, String> {
    let tag_query = if tags.is_empty() {
        "music".to_string()
    } else {
        tags.join(",")
    };

    // Try multiple API servers with different endpoints
    let api_servers = vec![
        "https://nl1.api.radio-browser.info",
        "https://de1.api.radio-browser.info",
        "https://at1.api.radio-browser.info",
    ];

    let encoded_tag = urlencoding::encode(&tag_query);

    for server in api_servers {
        // Try the search endpoint first
        let url = format!(
            "{}/json/stations/search?tag={}&hidebroken=true&order=votes&reverse=true&limit=100",
            server, encoded_tag
        );

        eprintln!("Trying to fetch from: {}", url);

        match try_fetch_stations(&url).await {
            Ok(stations) => {
                eprintln!("Successfully loaded {} stations", stations.len());
                return Ok(stations);
            }
            Err(e) => {
                eprintln!("Failed with {}: {}", url, e);
            }
        }
    }

    // If all servers failed, return error
    Err("Could not reach radio-browser.info API from any server. Please check your internet connection.".to_string())
}

async fn try_fetch_stations(url: &str) -> Result<Vec<RadioStation>, String> {
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "vasak-resonance/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Request error: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Error reading response: {}", e))?;

    eprintln!("Response preview: {}", &response_text.chars().take(300).collect::<String>());

    let stations: Vec<RadioStation> = serde_json::from_str(&response_text)
        .map_err(|e| {
            format!("JSON parse error: {}. Response: {}", e, &response_text.chars().take(200).collect::<String>())
        })?;

    // Filter out stations with empty name or url
    let valid_stations = stations
        .into_iter()
        .filter(|s| !s.name.trim().is_empty() && !s.url.trim().is_empty())
        .collect::<Vec<_>>();

    if valid_stations.is_empty() {
        return Err("No valid stations in response".to_string());
    }

    Ok(valid_stations)
}

/// Parses .m3u playlist format and extracts stream URLs
pub fn parse_m3u(content: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            urls.push(trimmed.to_string());
        }
    }
    urls
}

/// Parses .pls playlist format and extracts stream URLs
pub fn parse_pls(content: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for line in content.lines() {
        if line.starts_with("File") && line.contains('=') {
            if let Some(url) = line.split('=').nth(1) {
                let trimmed = url.trim();
                if !trimmed.is_empty() {
                    urls.push(trimmed.to_string());
                }
            }
        }
    }
    urls
}

/// Fetches playlist from URL and extracts stream URL
pub async fn resolve_stream_url(playlist_url: &str) -> Result<String, String> {
    let response = reqwest::Client::new()
        .get(playlist_url)
        .send()
        .await
        .map_err(|e| format!("Error fetching playlist: {}", e))?;

    let content = response
        .text()
        .await
        .map_err(|e| format!("Error reading playlist: {}", e))?;

    // Try to parse as m3u first
    let urls = parse_m3u(&content);
    if !urls.is_empty() {
        return Ok(urls[0].clone());
    }

    // Try to parse as pls
    let urls = parse_pls(&content);
    if !urls.is_empty() {
        return Ok(urls[0].clone());
    }

    // If it looks like a direct URL, return it
    if playlist_url.starts_with("http://") || playlist_url.starts_with("https://") {
        return Ok(playlist_url.to_string());
    }

    Err("Could not resolve stream URL from playlist".to_string())
}

/// Trait for different audio source types (local file, stream, etc.)
pub trait AudioSource: Send + Sync + Read {
    fn as_read(&mut self) -> &mut dyn Read;
}

/// Implementation of AudioSource for HTTP streams (Icecast, Shoutcast, etc.)
pub struct IcecastStream {
    inner: Box<dyn Read + Send + Sync>,
}

impl IcecastStream {
    /// Creates a new IcecastStream from a URL
    pub async fn from_url(url: &str) -> Result<Self, String> {
        let bytes = reqwest::Client::new()
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Error connecting to stream: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("Error reading stream: {}", e))?;

        let cursor = std::io::Cursor::new(bytes.to_vec());
        let reader = Box::new(cursor) as Box<dyn Read + Send + Sync>;
        Ok(IcecastStream { inner: reader })
    }
}

impl Read for IcecastStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl AudioSource for IcecastStream {
    fn as_read(&mut self) -> &mut dyn Read {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_m3u() {
        let m3u = "#EXTM3U\n#EXTINF:-1,Station Name\nhttp://stream.example.com:8000/radio\n";
        let urls = parse_m3u(m3u);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "http://stream.example.com:8000/radio");
    }

    #[test]
    fn test_parse_pls() {
        let pls = "[playlist]\nFile1=http://stream.example.com:8000/radio\nTitle1=Station Name\nLength1=-1\nNumberOfEntries=1\nVersion=2";
        let urls = parse_pls(pls);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "http://stream.example.com:8000/radio");
    }

    #[test]
    fn test_radio_station_tag_list() {
        let station = RadioStation {
            uuid: "test".to_string(),
            name: "Test Radio".to_string(),
            url: "http://example.com".to_string(),
            homepage: None,
            favicon: None,
            tags: Some("lofi,relaxing,indie".to_string()),
            country: None,
            state: None,
            language: None,
            votes: None,
            codec: None,
            bitrate: None,
        };
        let tags = station.tag_list();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"lofi".to_string()));
    }
}
