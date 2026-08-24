mod audio;
mod audio_manager;
mod commands;
mod db;
mod discord;
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
use commands::reveal::show_in_file_manager;
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

/// Dónde viven las traducciones.
///
/// El plugin de i18n sólo prueba rutas relativas al ejecutable y al directorio
/// de trabajo, y ninguna de esas existe cuando el binario está instalado en
/// `/usr/bin` — un build empaquetado mostraría las claves crudas. Resolverla
/// acá y pasarla explícitamente cubre el árbol de desarrollo y la ubicación
/// instalada. Es el mismo patrón que usa vasak-settings.
fn locales_dir() -> Option<String> {
    let candidates = [
        std::path::PathBuf::from("locales"),
        std::path::PathBuf::from("src-tauri/locales"),
        std::path::PathBuf::from("/usr/share/vasak-resonance/locales"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .map(|path| path.to_string_lossy().into_owned())
}

/// Elige el idioma inicial según el de la sesión, con español por omisión:
/// es el idioma con el que la interfaz venía escrita antes de ser traducible.
fn default_locale() -> String {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    let code = raw
        .split('.')
        .next()
        .unwrap_or_default()
        .split('_')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if code == "en" {
        "en".to_string()
    } else {
        "es".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    ensure_vasak_config_dir();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_i18n_vsk::init_with_path(
            Some(default_locale()),
            locales_dir(),
        ))
        // Sólo para que «Copiar», «Cortar» y «Pegar» del menú hagan de verdad lo
        // que dicen: el motor del navegador no da acceso al portapapeles desde
        // la página.
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_vsk_contextual_menu::init())
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

                // La presencia en Discord: el hilo arranca acá y se queda
                // esperando. Si no hay identificador configurado no arranca
                // nada y la aplicación no se entera.
                app.manage(discord::DiscordPresence::iniciar());

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
            show_in_file_manager,
            commands::discord::update_discord_presence,
            commands::discord::clear_discord_presence,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, evento| {
            // Salir sin limpiar deja el perfil diciendo que seguís escuchando
            // algo que ya no suena, hasta que Discord se cierre.
            if matches!(evento, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
                if let Some(presencia) = app.try_state::<discord::DiscordPresence>() {
                    presencia.cerrar();
                }
            }
        });
}
