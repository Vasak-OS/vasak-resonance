mod audio;
mod audio_manager;
mod commands;
mod db;
mod structs;

use audio_manager::AudioState;
use commands::audio_control::{pause, play_file, resume, seek, set_volume};
use commands::indexing::scan_music_folders;
use commands::playback::handle_dropped_file;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .setup(move |app| {
            let audio_state = AudioState::new(app.handle().clone());
            app.manage(audio_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_music_folders,
            handle_dropped_file,
            play_file,
            pause,
            resume,
            seek,
            set_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
