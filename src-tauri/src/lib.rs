mod audio;
mod commands;
mod db;
mod structs;

use commands::indexing::scan_music_folders;
use commands::playback::handle_dropped_file;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .invoke_handler(tauri::generate_handler![scan_music_folders, handle_dropped_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
