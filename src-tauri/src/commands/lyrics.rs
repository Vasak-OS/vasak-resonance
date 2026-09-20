use std::path::Path;

use crate::lyrics::{fetch_track_lyrics, letras_del_disco, LyricsQuery};
use crate::structs::TrackLyricsPayload;

/// Las letras de un tema: primero las que están en el disco, después LRCLIB.
///
/// `path` es la ruta del archivo que suena, y puede no venir: una emisora de
/// radio no tiene archivo, y ahí el único camino es el de la red —que es el que
/// había siempre—.
///
/// Lo que sale del disco **no se cachea**: leerlo cuesta un `stat` y una lectura
/// de unos kilobytes, y guardarlo haría que editar el `.lrc` no se viera hasta
/// vaciar la caché.
#[tauri::command]
pub async fn fetch_lyrics(
    track_name: String,
    artist_name: String,
    album_name: String,
    duration_seconds: u64,
    path: Option<String>,
) -> Result<TrackLyricsPayload, String> {
    if let Some(path) = path.as_deref().filter(|path| !path.is_empty()) {
        if let Some(letras) = letras_del_disco(Path::new(path)) {
            return Ok(letras);
        }
    }

    fetch_track_lyrics(LyricsQuery {
        track_name,
        artist_name,
        album_name,
        duration_seconds,
    })
    .await
}
