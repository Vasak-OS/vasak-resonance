use std::collections::HashMap;

use tauri::{AppHandle, Emitter};
use zbus::fdo;
use zbus::zvariant::{OwnedValue, Value};
use zbus::ConnectionBuilder;

use crate::audio_manager::AudioState;

const MPRIS_BUS_NAME: &str = "org.mpris.MediaPlayer2.vasak-resonance";
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
        self.audio_state.play().map_err(fdo::Error::Failed)
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

    fn previous(&self) -> fdo::Result<()> {
        self.app_handle
            .emit("mpris-previous-request", ())
            .map_err(|e| fdo::Error::Failed(format!("No se pudo emitir evento previous: {e}")))
    }

    fn stop(&self) -> fdo::Result<()> {
        self.audio_state
            .stop()
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    #[zbus(property)]
    fn playback_status(&self) -> fdo::Result<String> {
        self.audio_state
            .playback_status()
            .map(|s| s.to_string())
            .map_err(fdo::Error::Failed)
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let snapshot = self.audio_state.playback_snapshot().unwrap_or_default();
        build_metadata(&snapshot)
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        "None".to_string()
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        let snapshot = self.audio_state.playback_snapshot().unwrap_or_default();
        (snapshot.position_seconds as i64).saturating_mul(1_000_000)
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        let snapshot = self.audio_state.playback_snapshot().unwrap_or_default();
        snapshot.volume as f64
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        false
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
    fn can_stop(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.audio_state
            .playback_snapshot()
            .map(|snapshot| snapshot.path.is_some())
            .unwrap_or(false)
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

fn build_metadata(snapshot: &crate::structs::PlaybackProgressEvent) -> HashMap<String, OwnedValue> {
    let mut metadata = HashMap::new();

    let insert_value = |map: &mut HashMap<String, OwnedValue>, key: &str, value: Value<'static>| {
        if let Ok(owned_value) = OwnedValue::try_from(value) {
            map.insert(key.to_string(), owned_value);
        }
    };

    if let Some(path) = &snapshot.path {
        insert_value(
            &mut metadata,
            "xesam:url",
            Value::new(format!("file://{}", path)),
        );
        insert_value(
            &mut metadata,
            "mpris:trackid",
            Value::new("/org/mpris/MediaPlayer2/TrackList/NoTrack"),
        );
    }

    if let Some(now_playing) = &snapshot.now_playing {
        insert_value(
            &mut metadata,
            "xesam:title",
            Value::new(now_playing.title.clone()),
        );
        insert_value(
            &mut metadata,
            "xesam:artist",
            Value::new(vec![now_playing.artist.clone()]),
        );
        insert_value(
            &mut metadata,
            "xesam:album",
            Value::new(now_playing.album.clone()),
        );
        insert_value(
            &mut metadata,
            "mpris:length",
            Value::new((now_playing.duration_seconds as i64) * 1_000_000),
        );

        if let Some(cover_data_url) = &now_playing.cover_data_url {
            insert_value(
                &mut metadata,
                "mpris:artUrl",
                Value::new(cover_data_url.clone()),
            );
        }
    }

    metadata
}
