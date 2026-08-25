//! El puente con la presencia de Discord.
//!
//! Los comandos no esperan a Discord: dejan el mensaje en el canal del hilo que
//! habla con el socket y vuelven. Ese es todo el punto —la interfaz avisa cada
//! vez que cambia la canción, y avisar no puede costar nada—.

use tauri::State;

use crate::discord::{DiscordPresence, Presencia};

#[tauri::command]
pub async fn update_discord_presence(
    presencia: State<'_, DiscordPresence>,
    title: String,
    artist: String,
    album_art_url: Option<String>,
    is_paused: bool,
    duration_secs: u64,
    current_time_secs: u64,
) -> Result<(), String> {
    presencia.actualizar(Presencia {
        title,
        artist,
        album_art_url,
        is_paused,
        duration_secs,
        current_time_secs,
    });

    Ok(())
}

/// Deja el perfil como estaba: al parar la música y al cerrar la aplicación.
#[tauri::command]
pub async fn clear_discord_presence(presencia: State<'_, DiscordPresence>) -> Result<(), String> {
    presencia.limpiar();
    Ok(())
}
