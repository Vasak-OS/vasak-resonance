use base64::{engine::general_purpose, Engine as _};
use color_thief::{get_palette, ColorFormat};
use image::ImageReader;
use lofty::picture::PictureType;
use lofty::prelude::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::structs::{NowPlayingMetadata, Track};

pub fn extract_track_from_file(path: &Path) -> Result<Track, String> {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let duration_seconds = tagged_file.properties().duration().as_secs() as i64;

    Ok(Track {
        id: None,
        path: canonical_path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_seconds,
    })
}

pub fn extract_now_playing_metadata(path: &Path) -> Result<NowPlayingMetadata, String> {
    let mut cover_cache = HashMap::<String, Option<String>>::new();
    let mut dominant_color_cache = HashMap::<String, Option<String>>::new();
    extract_now_playing_metadata_with_cover_cache(path, &mut cover_cache, &mut dominant_color_cache)
}

pub fn extract_now_playing_metadata_with_cover_cache(
    path: &Path,
    cover_cache: &mut HashMap<String, Option<String>>,
    dominant_color_cache: &mut HashMap<String, Option<String>>,
) -> Result<NowPlayingMetadata, String> {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let canonical_path_str = canonical_path.to_string_lossy().to_string();

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let duration_seconds = tagged_file.properties().duration().as_secs();

    let mut computed_cover_data_url: Option<String> = None;
    let mut computed_dominant_color: Option<String> = None;

    let (cover_data_url, dominant_color) = if let Some(cached_cover) =
        cover_cache.get(&canonical_path_str)
    {
        (
            cached_cover.clone(),
            dominant_color_cache
                .get(&canonical_path_str)
                .cloned()
                .unwrap_or(None),
        )
    } else {
        if let Some(tag) = primary_tag {
            if let Some(picture) = tag
                .get_picture_type(PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
            {
                let mime = picture
                    .mime_type()
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "image/jpeg".to_string());
                let encoded = general_purpose::STANDARD.encode(picture.data());
                computed_cover_data_url = Some(format!("data:{};base64,{}", mime, encoded));
                computed_dominant_color = extract_dominant_color_hex(picture.data());
            }
        }

        cover_cache.insert(canonical_path_str.clone(), computed_cover_data_url.clone());
        dominant_color_cache.insert(canonical_path_str.clone(), computed_dominant_color.clone());

        (computed_cover_data_url, computed_dominant_color)
    };

    Ok(NowPlayingMetadata {
        path: canonical_path_str,
        title,
        artist,
        album,
        duration_seconds,
        cover_data_url,
        dominant_color,
    })
}

fn extract_dominant_color_hex(image_data: &[u8]) -> Option<String> {
    let cursor = Cursor::new(image_data);
    let decoded = ImageReader::new(cursor)
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let rgba = decoded.to_rgba8();

    let palette = get_palette(rgba.as_raw(), ColorFormat::Rgba, 10, 5).ok()?;
    let color = palette.first()?;

    Some(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b))
}

pub fn is_supported_audio_file(path: &Path) -> bool {
    const SUPPORTED_EXTENSIONS: &[&str] = &[
        "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "opus", "wma", "aiff", "alac",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
