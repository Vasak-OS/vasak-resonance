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
