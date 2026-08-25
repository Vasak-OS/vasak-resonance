use serde::{Deserialize, Serialize};

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

    eprintln!("Response preview: {}", response_text.chars().take(300).collect::<String>());

    let stations: Vec<RadioStation> = serde_json::from_str(&response_text)
        .map_err(|e| {
            format!("JSON parse error: {}. Response: {}", e, response_text.chars().take(200).collect::<String>())
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
