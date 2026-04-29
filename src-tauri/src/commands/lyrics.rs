use crate::lyrics::{fetch_track_lyrics, LyricsQuery};
use crate::structs::TrackLyricsPayload;

#[tauri::command]
pub async fn fetch_lyrics(
    track_name: String,
    artist_name: String,
    album_name: String,
    duration_seconds: u64,
) -> Result<TrackLyricsPayload, String> {
    fetch_track_lyrics(LyricsQuery {
        track_name,
        artist_name,
        album_name,
        duration_seconds,
    })
    .await
}
