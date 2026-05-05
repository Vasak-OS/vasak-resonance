use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use base64::Engine;
const DEEZER_BASE_URL: &str = "https://api.deezer.com";
const MUSICBRAINZ_BASE_URL: &str = "https://musicbrainz.org/ws/2";
const COVERARTARCHIVE_BASE_URL: &str = "https://coverartarchive.org";

const ALBUM_CACHE_RELATIVE_PATH: &str = ".cache/resonance/albums";

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

fn album_cache_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)?;
    Some(home.join(ALBUM_CACHE_RELATIVE_PATH))
}

/// Normalize album name: lowercase, alphanumeric + hyphens only
fn normalize_album_name(album: &str) -> String {
    album
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .trim_matches('-')
        .to_string()
}

/// Get cached album cover path if it exists
fn get_cached_cover_path(artist: &str, album: &str) -> Option<PathBuf> {
    let cache_dir = album_cache_dir()?;
    let normalized = format!("{}-{}", normalize_album_name(artist), normalize_album_name(album));
    
    // Check for .jpg or .png
    let jpg_path = cache_dir.join(format!("{}.jpg", normalized));
    if jpg_path.exists() {
        return Some(jpg_path);
    }
    
    let png_path = cache_dir.join(format!("{}.png", normalized));
    if png_path.exists() {
        return Some(png_path);
    }
    
    None
}

#[derive(Debug, Deserialize)]
struct DeezerAlbumSearchResult {
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    title: String,
    cover_big: Option<String>,
    cover_xl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeezerSearchResponse {
    data: Vec<DeezerAlbumSearchResult>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzRelease {
    id: String,
    #[allow(dead_code)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzSearchResponse {
    releases: Vec<MusicBrainzRelease>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct CoverArtArchiveResponse {
    images: Vec<CoverImage>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct CoverImage {
    front: bool,
    back: bool,
    image: String,
}

/// Try to fetch album cover from Deezer API
async fn fetch_from_deezer(artist: &str, album: &str) -> Option<String> {
    let query = format!("{} {}", artist, album);
    let url = format!("{}/search/album?q={}", DEEZER_BASE_URL, urlencoding::encode(&query));
    
    let response = http_client()
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    
    let data: DeezerSearchResponse = response.json().await.ok()?;
    
    // Get first result with cover image
    data.data
        .iter()
        .find(|album_result| album_result.cover_xl.is_some() || album_result.cover_big.is_some())
        .and_then(|album_result| {
            album_result.cover_xl.as_ref().or(album_result.cover_big.as_ref()).cloned()
        })
}

/// Try to fetch album cover from MusicBrainz API (via CoverArtArchive)
async fn fetch_from_musicbrainz(artist: &str, album: &str) -> Option<String> {
    // Step 1: Search for release on MusicBrainz
    let query = format!("artist:{} AND release:{}", artist, album);
    let url = format!(
        "{}/release?query={}&fmt=json",
        MUSICBRAINZ_BASE_URL,
        urlencoding::encode(&query)
    );
    
    let response = http_client()
        .get(&url)
        .header("User-Agent", "vasak-resonance/1.0")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    
    let data: MusicBrainzSearchResponse = response.json().await.ok()?;
    
    let release_id = data.releases.first()?.id.clone();
    
    // Step 2: Get cover art from CoverArtArchive
    let cover_url = format!("{}/release/{}/front", COVERARTARCHIVE_BASE_URL, release_id);
    
    // Check if cover exists with HEAD request
    let head_response = http_client()
        .head(&cover_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    
    if head_response.status().is_success() {
        return Some(cover_url);
    }
    
    None
}

/// Download image from URL and return bytes
async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let response = http_client()
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read image bytes: {}", e))
}

/// Determine image format from response headers or URL
fn guess_image_format(content_type: Option<&str>, url: &str) -> &'static str {
    if let Some(ct) = content_type {
        if ct.contains("png") {
            return "png";
        }
        if ct.contains("jpeg") || ct.contains("jpg") {
            return "jpg";
        }
    }
    
    // Fallback: guess from URL
    if url.contains(".png") {
        "png"
    } else {
        "jpg"
    }
}

/// Convert image file to data URL (base64 encoded)
fn file_to_data_url(path: &PathBuf, format: &str) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Failed to read image file: {}", e))?;
	
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mime_type = match format {
        "png" => "image/png",
        _ => "image/jpeg",
    };
	
    Ok(format!("data:{};base64,{}", mime_type, encoded))
}
/// Main function: Fetch and cache album cover
pub async fn fetch_album_cover(artist: String, album: String) -> Result<String, String> {
    // Check cache first
    if let Some(cached_path) = get_cached_cover_path(&artist, &album) {
        // Convert cached file to data URL
        let format = if cached_path.extension().and_then(|ext| ext.to_str()) == Some("png") {
            "png"
        } else {
            "jpg"
        };
        return file_to_data_url(&cached_path, format);
    }
    
    // Try Deezer first (faster, single request)
    let image_url = if let Some(url) = fetch_from_deezer(&artist, &album).await {
        url
    } else if let Some(url) = fetch_from_musicbrainz(&artist, &album).await {
        // MusicBrainz as fallback
        url
    } else {
        return Err("Could not find album cover on Deezer or MusicBrainz".to_string());
    };
    
    // Download image
    let image_bytes = download_image(&image_url).await?;
    
    // Determine format
    let format = guess_image_format(None, &image_url);
    
    // Ensure cache directory exists
    let cache_dir = album_cache_dir()
        .ok_or_else(|| "Could not determine cache directory".to_string())?;
    
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    
    // Generate cache filename
    let normalized = format!(
        "{}-{}",
        normalize_album_name(&artist),
        normalize_album_name(&album)
    );
    let cache_file = cache_dir.join(format!("{}.{}", normalized, format));
    
    // Write to cache
    fs::write(&cache_file, image_bytes)
        .map_err(|e| format!("Failed to write image to cache: {}", e))?;
    
    // Return as data URL
    file_to_data_url(&cache_file, format)
}
