use tauri::{AppHandle, Emitter};
use zbus::fdo;
use zbus::ConnectionBuilder;

use crate::audio_manager::AudioState;

const MPRIS_BUS_NAME: &str = "org.mpris.MediaPlayer2.vasakresonance";
const MPRIS_OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";

pub fn start_mpris_service(app_handle: AppHandle, audio_state: AudioState) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_mpris_service(app_handle, audio_state).await {
            eprintln!("MPRIS no pudo iniciarse: {error}");
        }
    });
}

async fn run_mpris_service(app_handle: AppHandle, audio_state: AudioState) -> Result<(), String> {
    let root_iface = MprisRootInterface;
    let player_iface = MprisPlayerInterface {
        app_handle,
        audio_state,
    };

    let _connection = ConnectionBuilder::session()
        .map_err(|e| format!("No se pudo abrir bus de sesión: {e}"))?
        .name(MPRIS_BUS_NAME)
        .map_err(|e| format!("No se pudo registrar nombre MPRIS: {e}"))?
        .serve_at(MPRIS_OBJECT_PATH, root_iface)
        .map_err(|e| format!("No se pudo registrar interfaz raíz MPRIS: {e}"))?
        .serve_at(MPRIS_OBJECT_PATH, player_iface)
        .map_err(|e| format!("No se pudo registrar interfaz player MPRIS: {e}"))?
        .build()
        .await
        .map_err(|e| format!("No se pudo construir conexión MPRIS: {e}"))?;

    std::future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}

struct MprisRootInterface;

struct MprisPlayerInterface {
    app_handle: AppHandle,
    audio_state: AudioState,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl MprisRootInterface {
    fn raise(&self) {}

    fn quit(&self) {}

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> String {
        "Vasak Resonance".to_string()
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["file".to_string()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "audio/mpeg".to_string(),
            "audio/flac".to_string(),
            "audio/ogg".to_string(),
            "audio/wav".to_string(),
            "audio/aac".to_string(),
            "audio/mp4".to_string(),
        ]
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayerInterface {
    fn play(&self) -> fdo::Result<()> {
        match self.audio_state.playback_status() {
            Ok("Paused") => self
                .audio_state
                .resume()
                .map_err(|e| fdo::Error::Failed(e.to_string())),
            Ok(_) => Ok(()),
            Err(e) => Err(fdo::Error::Failed(e)),
        }
    }

    fn pause(&self) -> fdo::Result<()> {
        self.audio_state
            .pause()
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    fn play_pause(&self) -> fdo::Result<()> {
        self.audio_state
            .play_pause_toggle()
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    fn next(&self) -> fdo::Result<()> {
        self.app_handle
            .emit("mpris-next-request", ())
            .map_err(|e| fdo::Error::Failed(format!("No se pudo emitir evento next: {e}")))
    }

    #[zbus(property)]
    fn playback_status(&self) -> fdo::Result<String> {
        self.audio_state
            .playback_status()
            .map(|s| s.to_string())
            .map_err(fdo::Error::Failed)
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}
