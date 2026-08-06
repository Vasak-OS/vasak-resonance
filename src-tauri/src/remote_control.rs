//! WebSocket remote-control service.
//!
//! Security model: the service binds to **loopback only** by default, so it is
//! not reachable from the network. Exposing it to the LAN is opt-in *and*
//! requires a shared token — the service refuses to bind a non-loopback address
//! without one. When a token is configured, a client must authenticate before
//! any command is accepted.
//!
//! Configuration lives in `~/.config/vasak-resonance/remote.json` (all fields
//! optional):
//!
//! ```json
//! {
//!   "enabled": true,
//!   "bind_lan": false,
//!   "port": 30123,
//!   "token": null
//! }
//! ```
//!
//! To control playback from another device on the LAN, set `bind_lan` to true
//! and `token` to a non-empty secret; the first WebSocket message must then be
//! `{"auth":"<token>"}`.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::audio_manager::AudioState;

const DEFAULT_PORT: u16 = 30123;

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RemoteConfig {
    /// Whether the remote-control service runs at all.
    enabled: bool,
    /// Bind to `0.0.0.0` (LAN) instead of loopback. Ignored unless a token is set.
    bind_lan: bool,
    /// TCP port to listen on.
    port: u16,
    /// Shared secret required to authenticate clients. Mandatory for LAN exposure.
    token: Option<String>,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_lan: false,
            port: DEFAULT_PORT,
            token: None,
        }
    }
}

fn load_remote_config() -> RemoteConfig {
    let Some(base) = dirs::config_dir() else {
        return RemoteConfig::default();
    };
    let path = base.join("vasak-resonance/remote.json");
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            eprintln!(
                "[resonance] remote.json inválido ({error}); usando valores por defecto seguros"
            );
            RemoteConfig::default()
        }),
        // No config file: safe defaults (loopback, no token).
        Err(_) => RemoteConfig::default(),
    }
}

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

#[derive(Debug, Deserialize)]
struct AuthMessage {
    auth: String,
}

/// Constant-time byte comparison to avoid leaking the token via timing.
/// (The length check leaks only the token length, which is acceptable.)
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
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
    let config = load_remote_config();
    if !config.enabled {
        eprintln!("[resonance] control remoto deshabilitado por configuración");
        return Ok(());
    }

    let token = config
        .token
        .filter(|value| !value.is_empty())
        .map(Arc::new);

    // Never expose the service beyond loopback without a token.
    let bind_ip = if config.bind_lan {
        if token.is_some() {
            "0.0.0.0"
        } else {
            eprintln!(
                "[resonance] bind_lan=true sin token; se fuerza loopback por seguridad"
            );
            "127.0.0.1"
        }
    } else {
        "127.0.0.1"
    };

    let address = format!("{bind_ip}:{}", config.port);
    let listener = TcpListener::bind(&address)
        .await
        .map_err(|error| format!("No se pudo abrir el puerto WebSocket en {address}: {error}"))?;

    eprintln!(
        "[resonance] control remoto escuchando en {address} (auth {})",
        if token.is_some() { "requerida" } else { "no requerida" }
    );

    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .map_err(|error| format!("No se pudo aceptar conexión WebSocket: {error}"))?;

        let app_handle = app_handle.clone();
        let audio_state = audio_state.clone();
        let token = token.clone();

        tauri::async_runtime::spawn(async move {
            if let Err(error) = handle_connection(stream, app_handle, audio_state, token).await {
                eprintln!("Error en conexión WebSocket desde {peer_addr}: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    app_handle: AppHandle,
    audio_state: AudioState,
    token: Option<Arc<String>>,
) -> Result<(), String> {
    let mut websocket = accept_async(stream)
        .await
        .map_err(|error| format!("No se pudo negociar WebSocket: {error}"))?;

    // With a token configured, the client must authenticate before any command.
    let mut authenticated = token.is_none();

    while let Some(message) = websocket.next().await {
        let message = message.map_err(|error| format!("Error de lectura WebSocket: {error}"))?;

        match message {
            Message::Text(text) => {
                if !authenticated {
                    let ok = match serde_json::from_str::<AuthMessage>(&text) {
                        Ok(auth) => token
                            .as_deref()
                            .map(|expected| {
                                constant_time_eq(auth.auth.as_bytes(), expected.as_bytes())
                            })
                            .unwrap_or(true),
                        Err(_) => false,
                    };

                    if ok {
                        authenticated = true;
                        websocket
                            .send(Message::Text(json!({"ok": true}).to_string()))
                            .await
                            .map_err(|error| format!("No se pudo responder por WebSocket: {error}"))?;
                    } else {
                        // Reject and close: never process commands unauthenticated.
                        let _ = websocket
                            .send(
                                Message::Text(
                                    json!({"ok": false, "error": "autenticación requerida"})
                                        .to_string(),
                                ),
                            )
                            .await;
                        break;
                    }
                    continue;
                }

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
