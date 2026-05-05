use tauri::State;
use crate::audio_manager::AudioState;
use crate::radio::{fetch_stations, RadioStation};

/// Fetches radio stations from radio-browser.info API filtered by tags
#[tauri::command]
pub async fn fetch_radio_stations(tags: Vec<String>) -> Result<Vec<RadioStation>, String> {
    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    fetch_stations(tag_refs).await
}

/// Plays a radio stream
#[tauri::command]
pub fn play_radio_stream(
    audio_state: State<'_, AudioState>,
    url: String,
    station_name: String,
) -> Result<(), String> {
    audio_state.play_stream(url, station_name)
}
