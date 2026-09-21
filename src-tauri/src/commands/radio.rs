use tauri::{AppHandle, State};
use tauri_plugin_vsk_journal::DiarioExt;

use crate::audio_manager::AudioState;
use crate::radio::{fetch_stations, RadioStation};

/// Busca emisoras en el directorio, filtradas por etiqueta.
///
/// Lo que salga mal va al diario del sistema y no a la salida de error: la
/// aplicación se abre desde el escritorio, así que un `eprintln!` no lo lee
/// nadie, y acá adentro lo que se imprimía incluía trescientos caracteres de la
/// respuesta cruda del directorio.
#[tauri::command]
pub async fn fetch_radio_stations(
    app: AppHandle,
    tags: Vec<String>,
) -> Result<Vec<RadioStation>, String> {
    let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();

    match fetch_stations(tag_refs).await {
        Ok(estaciones) => {
            app.diario()
                .informacion(&format!("radio: {} emisoras", estaciones.len()));
            Ok(estaciones)
        }
        Err(error) => {
            app.diario().aviso(&format!("radio: {error}"));
            Err(error)
        }
    }
}

/// Plays a radio stream
#[tauri::command]
pub fn play_radio_stream(
    audio_state: State<'_, AudioState>,
    url: String,
    station_name: String,
) -> Result<(), String> {
    audio_state.play_stream(url, station_name)
}
