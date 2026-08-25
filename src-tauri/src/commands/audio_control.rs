use tauri::State;

use crate::audio_manager::AudioState;
use crate::structs::PlaybackProgressEvent;

#[tauri::command]
pub fn play_file(file_path: String, seek_to: Option<u64>, state: State<AudioState>) -> Result<(), String> {
    state.play_file(file_path, seek_to)
}

#[tauri::command]
pub fn pause(state: State<AudioState>) -> Result<(), String> {
    state.pause()
}

#[tauri::command]
pub fn stop(state: State<AudioState>) -> Result<(), String> {
    state.stop()
}

#[tauri::command]
pub fn resume(state: State<AudioState>) -> Result<(), String> {
    state.resume()
}

#[tauri::command]
pub fn seek(second: u64, state: State<AudioState>) -> Result<(), String> {
    state.seek(second)
}

#[tauri::command]
pub fn set_volume(volume: f32, state: State<AudioState>) -> Result<(), String> {
    state.set_volume(volume)
}

#[tauri::command]
pub fn get_playback_snapshot(state: State<AudioState>) -> Result<PlaybackProgressEvent, String> {
    state.playback_snapshot()
}

/// Sets how long tracks overlap when one ends and the next begins.
///
/// Zero turns the overlap off, which is what someone listening to records that
/// segue wants: any crossfade destroys the join.
#[tauri::command]
pub fn set_crossfade(seconds: f32, state: State<AudioState>) -> Result<(), String> {
    state.set_crossfade(seconds)
}

/// Tells the audio thread which track comes next so it can start the crossfade
/// without waiting to be asked.
///
/// The queue lives in the frontend store, so the backend cannot work this out
/// on its own — and by the time an "the track ended" round trip completes, the
/// moment to overlap has passed. `None` means nothing follows.
#[tauri::command]
pub fn set_next_track(file_path: Option<String>, state: State<AudioState>) -> Result<(), String> {
    state.set_next_track(file_path)
}
