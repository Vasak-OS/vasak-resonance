use base64::{engine::general_purpose, Engine as _};
use lofty::picture::PictureType;
use lofty::prelude::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::fs;
use std::path::Path;

use crate::structs::{NowPlayingMetadata, Track};

pub fn extract_track_from_file(path: &Path) -> Result<Track, String> {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
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
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
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

    let cover_data_url = primary_tag.and_then(|tag| {
        let picture = tag
            .get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())?;

        let mime = picture
            .mime_type()
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "image/jpeg".to_string());
        let encoded = general_purpose::STANDARD.encode(picture.data());
        Some(format!("data:{};base64,{}", mime, encoded))
    });

    Ok(NowPlayingMetadata {
        path: canonical_path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_seconds,
        cover_data_url,
    })
}

pub fn is_supported_audio_file(path: &Path) -> bool {
    const SUPPORTED_EXTENSIONS: &[&str] = &[
        "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "opus", "wma", "aiff",
        "alac",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
