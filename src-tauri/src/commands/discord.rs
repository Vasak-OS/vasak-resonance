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

/// Si la presencia está encendida.
///
/// La interfaz lo pregunta antes de salir a buscar la tapa del álbum: esa
/// búsqueda le manda el artista y el álbum a un servicio ajeno, y no hay motivo
/// para hacerlo en un equipo donde la presencia está apagada —que es el caso por
/// omisión, porque hace falta configurar un identificador de aplicación—.
#[tauri::command]
pub async fn discord_presence_activa(
    presencia: State<'_, DiscordPresence>,
) -> Result<bool, String> {
    Ok(presencia.esta_activa())
}

/// Deja el perfil como estaba: al parar la música y al cerrar la aplicación.
#[tauri::command]
pub async fn clear_discord_presence(presencia: State<'_, DiscordPresence>) -> Result<(), String> {
    presencia.limpiar();
    Ok(())
}
