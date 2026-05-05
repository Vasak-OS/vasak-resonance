use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::audio_manager::AudioState;

const REMOTE_CONTROL_ADDRESS: &str = "0.0.0.0:30123";

#[derive(Debug, Deserialize)]
struct RemoteCommandRequest {
    command: RemoteCommand,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RemoteCommand {
    Play,
    Pause,
    Next,
}

pub fn start_remote_control_service(app_handle: AppHandle, audio_state: AudioState) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_remote_control_service(app_handle, audio_state).await {
            eprintln!("Control remoto por WebSocket no pudo iniciarse: {error}");
        }
    });
}

async fn run_remote_control_service(
    app_handle: AppHandle,
    audio_state: AudioState,
) -> Result<(), String> {
    let listener = TcpListener::bind(REMOTE_CONTROL_ADDRESS)
        .await
        .map_err(|error| format!("No se pudo abrir el puerto WebSocket: {error}"))?;

    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .map_err(|error| format!("No se pudo aceptar conexión WebSocket: {error}"))?;

        let app_handle = app_handle.clone();
        let audio_state = audio_state.clone();

        tauri::async_runtime::spawn(async move {
            if let Err(error) = handle_connection(stream, app_handle, audio_state).await {
                eprintln!("Error en conexión WebSocket desde {peer_addr}: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    app_handle: AppHandle,
    audio_state: AudioState,
) -> Result<(), String> {
    let mut websocket = accept_async(stream)
        .await
        .map_err(|error| format!("No se pudo negociar WebSocket: {error}"))?;

    while let Some(message) = websocket.next().await {
        let message = message.map_err(|error| format!("Error de lectura WebSocket: {error}"))?;

        match message {
            Message::Text(text) => {
                let response = handle_command_text(&text, &app_handle, &audio_state).await;
                websocket
                    .send(Message::Text(response))
                    .await
                    .map_err(|error| format!("No se pudo responder por WebSocket: {error}"))?;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}

async fn handle_command_text(
    text: &str,
    app_handle: &AppHandle,
    audio_state: &AudioState,
) -> String {
    match serde_json::from_str::<RemoteCommandRequest>(text) {
        Ok(request) => {
            let result = match request.command {
                RemoteCommand::Play => audio_state.play(),
                RemoteCommand::Pause => audio_state.pause(),
                RemoteCommand::Next => app_handle
                    .emit("mpris-next-request", ())
                    .map_err(|error| format!("No se pudo emitir la acción next: {error}")),
            };

            match result {
                Ok(()) => json!({"ok": true}).to_string(),
                Err(error) => json!({"ok": false, "error": error}).to_string(),
            }
        }
        Err(error) => json!({"ok": false, "error": format!("JSON inválido: {error}")}).to_string(),
    }
}