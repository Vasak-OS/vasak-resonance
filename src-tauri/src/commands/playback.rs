use std::path::PathBuf;

use crate::audio::{extract_now_playing_metadata, is_supported_audio_file};
use crate::structs::DroppedPlaybackTrack;

#[tauri::command]
pub fn handle_dropped_file(file_path: String) -> Result<DroppedPlaybackTrack, String> {
    let path = PathBuf::from(file_path);

    if !path.exists() || !path.is_file() {
        return Err("El archivo soltado no existe o no es válido".to_string());
    }

    if !is_supported_audio_file(&path) {
        return Err("El archivo soltado no es un formato de audio soportado".to_string());
    }

    let track = extract_now_playing_metadata(&path)?;

    Ok(DroppedPlaybackTrack {
        path: track.path,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration_seconds: track.duration_seconds as i64,
        cover_data_url: track.cover_data_url,
    })
}
