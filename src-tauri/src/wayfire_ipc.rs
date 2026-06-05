use serde_json::Value;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::sleep;

fn find_socket() -> Option<PathBuf> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")?;
    let runtime_dir = PathBuf::from(runtime_dir);

    for var in ["WAYFIRE_SOCKET", "WAYFIRE_IPC_SOCKET", "_WAYFIRE_SOCKET"] {
        if let Some(path) = env::var_os(var) {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    if let Some(display) = env::var("WAYLAND_DISPLAY").ok() {
        let p = runtime_dir.join(format!("wayfire-{}-.socket", display));
        if p.exists() {
            return Some(p);
        }
    }

    for name in &["wayfire.socket", "wayfire-ipc.socket", "wayfire-ipc.sock"] {
        let p = runtime_dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

async fn connect() -> Result<UnixStream, String> {
    let socket_path = find_socket()
        .ok_or_else(|| "No se pudo localizar el socket de Wayfire (¿está corriendo Wayfire?)".to_string())?;

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match UnixStream::connect(&socket_path).await {
            Ok(s) => return Ok(s),
            Err(_) if std::time::Instant::now() < deadline => {
                sleep(Duration::from_millis(200)).await
            }
            Err(e) => return Err(format!("No se pudo conectar a Wayfire IPC: {e}")),
        }
    }
}

async fn send_request(
    stream: &mut UnixStream,
    method: &str,
    data: Value,
) -> Result<Value, String> {
    let payload = serde_json::json!({ "method": method, "data": data });
    let serialized = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let len = serialized.len() as u32;

    let mut header = [0u8; 4];
    header.copy_from_slice(&len.to_le_bytes());
    stream.write_all(&header).await.map_err(|e| e.to_string())?;
    stream.write_all(&serialized).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut resp_len = [0u8; 4];
    stream.read_exact(&mut resp_len).await.map_err(|e| e.to_string())?;
    let resp_size = u32::from_le_bytes(resp_len) as usize;

    let mut buf = vec![0u8; resp_size];
    stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;

    let response: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;

    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }

    Ok(response)
}

async fn find_view_id(
    stream: &mut UnixStream,
    title: &str,
) -> Result<i64, String> {
    let result = send_request(stream, "window-rules/list-views", Value::Null).await?;
    result
        .as_array()
        .and_then(|arr| {
            arr.iter().find(|v| {
                v.get("title")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t == title)
            })
        })
        .and_then(|v| v.get("id").and_then(Value::as_i64))
        .ok_or_else(|| format!("No se encontró \"{title}\" en Wayfire"))
}

pub async fn position_miniplayer(
    _target_x: i64,
    _target_y: i64,
    width: i64,
    height: i64,
) -> Result<(), String> {
    let mut stream = connect().await?;

    // Try to load window-rules plugin dynamically
    let _ = send_request(
        &mut stream,
        "plugin/load-plugin",
        serde_json::json!({ "name": "window-rules" }),
    )
    .await;

    // Get output geometry from Wayfire for correct coordinate space
    let result = send_request(&mut stream, "output/list-outputs", Value::Null).await?;
    let outputs = result.as_array().ok_or("output/list-outputs no devolvió un array")?;
    let out = outputs.first().ok_or("No hay outputs activos en Wayfire")?;
    let g = &out["geometry"];
    let ox = g["x"].as_i64().unwrap_or(0);
    let oy = g["y"].as_i64().unwrap_or(0);
    let ow = g["width"].as_i64().unwrap_or(0);
    let oh = g["height"].as_i64().unwrap_or(0);

    // Bottom-right corner with 10px margin
    let final_x = (ox + ow - width - 10).max(ox);
    let final_y = (oy + oh - height - 10).max(oy);

    // Find the view (retry up to 5s so the just-shown window appears)
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let view_id = loop {
        match find_view_id(&mut stream, "MiniPlayer - Resonance").await {
            Ok(id) => break id,
            Err(_) if std::time::Instant::now() < deadline => {
                sleep(Duration::from_millis(200)).await;
            }
            Err(e) => return Err(format!("Esperando ventana mini: {e}")),
        }
    };

    // Set sticky + always-on-top so it stays in place
    let _ = send_request(
        &mut stream,
        "wm-actions/set-sticky",
        serde_json::json!({ "view_id": view_id, "state": true }),
    ).await;

    let _ = send_request(
        &mut stream,
        "wm-actions/set-always-on-top",
        serde_json::json!({ "view_id": view_id, "state": true }),
    ).await;

    // Force position and size via window-rules
    send_request(
        &mut stream,
        "window-rules/configure-view",
        serde_json::json!({
            "id": view_id,
            "geometry": {
                "x": final_x,
                "y": final_y,
                "width": width,
                "height": height,
            },
        }),
    ).await?;

    Ok(())
}
