mod audio;
mod audio_manager;
mod commands;
mod db;
#[cfg(target_os = "linux")]
mod layer_shell;
mod lyrics;
mod metadata_fetcher;
#[cfg(target_os = "linux")]
mod mpris;
mod radio;
mod remote_control;
mod structs;

use audio_manager::AudioState;
use commands::audio_control::{
    get_playback_snapshot, pause, play_file, resume, seek, set_volume, stop,
};
use commands::indexing::{scan_default_music_folder, scan_music_folders};
use commands::library::{list_library_tracks, save_library_track, search_library_tracks};
use commands::lyrics::fetch_lyrics;
use commands::metadata::fetch_album_cover_command;
use commands::playback::handle_dropped_file;
use commands::playlists::{
    add_track_to_playlist_command, create_playlist_command, delete_playlist_command,
    list_playlist_tracks_command, list_playlists_command, remove_track_from_playlist_command,
};
use commands::radio::{fetch_radio_stations, play_radio_stream};
use commands::window::{toggle_main_and_miniplayer, close_app};
use tauri::Manager;

/// Makes sure the shared VasakOS configuration directory exists.
///
/// The config-manager plugin watches it and fails initialisation if it is
/// missing, which aborts startup: on a machine where nothing had created
/// `~/.config/vasak` yet, the player panicked before opening a window.
fn ensure_vasak_config_dir() {
    let Some(base) = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
    else {
        return;
    };

    let _ = std::fs::create_dir_all(base.join("vasak"));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ensure_vasak_config_dir();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .setup(move |app| {
            let audio_state = AudioState::new(app.handle().clone());
            #[cfg(target_os = "linux")]
            mpris::start_mpris_service(app.handle().clone(), audio_state.clone());

            #[cfg(target_os = "linux")]
            if let Some(mini_window) = app.get_webview_window("mini-player") {
                if let Ok(gtk_win) = mini_window.as_ref().window().gtk_window() {
                    layer_shell::setup_mini_player(gtk_win);
                }
            }

            remote_control::start_remote_control_service(app.handle().clone(), audio_state.clone());
                app.manage(audio_state.clone());

                let maybe_args: Vec<String> = std::env::args().skip(1).collect();
                if !maybe_args.is_empty() {
                    for raw in maybe_args.into_iter() {
                        let candidate = if raw.starts_with("file://") {
                            if raw.starts_with("file:///") {
                                raw.replacen("file://", "", 1)
                            } else if raw.starts_with("file://localhost/") {
                                raw.replacen("file://localhost", "", 1)
                            } else {
                                raw.replacen("file://", "", 1)
                            }
                        } else {
                            raw
                        };

                        let path = std::path::PathBuf::from(candidate);
                        if path.exists() && path.is_file() {
                            let play_path = path.to_string_lossy().to_string();
                            let audio_clone = audio_state.clone();
                            // Spawn so setup doesn't block; play_file will queue into audio thread.
                            std::thread::spawn(move || {
                                let _ = audio_clone.play_file(play_path, None);
                            });
                            break;
                        }
                    }
                }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_music_folders,
            scan_default_music_folder,
            list_library_tracks,
            save_library_track,
            search_library_tracks,
            handle_dropped_file,
            play_file,
            pause,
            stop,
            resume,
            seek,
            set_volume,
            get_playback_snapshot,
            fetch_lyrics,
            fetch_album_cover_command,
            create_playlist_command,
            list_playlists_command,
            delete_playlist_command,
            add_track_to_playlist_command,
            remove_track_from_playlist_command,
            list_playlist_tracks_command,
            fetch_radio_stations,
            play_radio_stream,
            toggle_main_and_miniplayer,
            close_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
