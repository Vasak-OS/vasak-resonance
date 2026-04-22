mod audio;
mod audio_manager;
mod commands;
mod db;
#[cfg(target_os = "linux")]
mod mpris;
mod structs;

use audio_manager::AudioState;
use commands::audio_control::{pause, play_file, resume, seek, set_volume};
use commands::indexing::scan_music_folders;
use commands::playlists::{
    add_track_to_playlist_command, create_playlist_command, delete_playlist_command,
    list_playlist_tracks_command, list_playlists_command, remove_track_from_playlist_command,
};
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
            #[cfg(target_os = "linux")]
            mpris::start_mpris_service(app.handle().clone(), audio_state.clone());
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
            set_volume,
            create_playlist_command,
            list_playlists_command,
            delete_playlist_command,
            add_track_to_playlist_command,
            remove_track_from_playlist_command,
            list_playlist_tracks_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
