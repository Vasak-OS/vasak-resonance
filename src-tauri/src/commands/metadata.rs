use crate::metadata_fetcher::{fetch_album_cover, PortadaConColor};

/// Baja la portada de un álbum y devuelve también su color dominante.
///
/// Los dos juntos y no sólo la imagen: los bytes ya están del lado de Rust, así
/// que calcular el color acá le ahorra al frontend decodificar la imagen otra
/// vez en un `<canvas>` para promediar píxeles a mano.
#[tauri::command]
pub async fn fetch_album_cover_command(
    artist: String,
    album: String,
) -> Result<PortadaConColor, String> {
    fetch_album_cover(artist, album).await
}
