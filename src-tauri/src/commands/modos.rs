//! El puente con los modos de reproducción.

use tauri::State;

use crate::modos::ModosDeReproduccion;

/// La ventana avisa cómo quedó el reproductor.
///
/// No cambia cómo se reproduce —la cola vive en la ventana—: lo que cambia es
/// lo que MPRIS contesta cuando el panel del escritorio pregunta.
#[tauri::command]
pub async fn set_playback_modes(
    modos: State<'_, ModosDeReproduccion>,
    repeticion: String,
    aleatorio: bool,
) -> Result<(), String> {
    modos.actualizar(&repeticion, aleatorio);
    Ok(())
}
