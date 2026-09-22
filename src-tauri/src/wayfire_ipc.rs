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
    let socket_path = find_socket().ok_or_else(|| {
        "No se pudo localizar el socket de Wayfire (¿está corriendo Wayfire?)".to_string()
    })?;

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

async fn send_request(stream: &mut UnixStream, method: &str, data: Value) -> Result<Value, String> {
    let payload = serde_json::json!({ "method": method, "data": data });
    let serialized = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let len = serialized.len() as u32;

    let mut header = [0u8; 4];
    header.copy_from_slice(&len.to_le_bytes());
    stream.write_all(&header).await.map_err(|e| e.to_string())?;
    stream
        .write_all(&serialized)
        .await
        .map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut resp_len = [0u8; 4];
    stream
        .read_exact(&mut resp_len)
        .await
        .map_err(|e| e.to_string())?;
    let resp_size = u32::from_le_bytes(resp_len) as usize;

    let mut buf = vec![0u8; resp_size];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| e.to_string())?;

    let response: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;

    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }

    Ok(response)
}

/// El identificador de la vista de un proceso dentro de `window-rules/list-views`.
///
/// Por pid **y** título. Sólo por pid no alcanza: el reproductor tiene dos
/// ventanas, la principal y el mini-reproductor, y las dos llevan el mismo pid.
/// Sólo por título tampoco: dos instancias abiertas tienen el mismo, y lo
/// escribe la página.
pub fn vista_del_proceso(vistas: &Value, pid: u32, titulo: &str) -> Option<i64> {
    vistas
        .as_array()?
        .iter()
        .find(|vista| {
            vista.get("pid").and_then(Value::as_u64) == Some(u64::from(pid))
                && vista.get("title").and_then(Value::as_str) == Some(titulo)
        })
        .and_then(|vista| vista.get("id").and_then(Value::as_i64))
}

/// Le pide al compositor que enfoque una ventana de este proceso.
///
/// Hace falta porque **en Wayland un cliente no se puede enfocar solo**. El
/// `set_focus` de Tauri devuelve `Ok` y el compositor lo ignora sin decir nada,
/// así que la ventana se muestra y se queda atrás igual. Se ve en Wayfire:
/// después de un `Raise` por MPRIS la vista sigue con `activated: false`, que
/// es por qué tocar el tema en el panel parecía no hacer nada.
///
/// Fuera de Wayfire no hay socket, y entonces esto no hace nada y no es un
/// error: la ventana ya se mostró, y lo que falta es que el compositor la
/// levante.
pub async fn enfocar_ventana_propia(titulo: String) -> Result<(), String> {
    if find_socket().is_none() {
        return Ok(());
    }

    let mut stream = connect().await?;
    let vistas = send_request(
        &mut stream,
        "window-rules/list-views",
        serde_json::json!({}),
    )
    .await?;

    let Some(id) = vista_del_proceso(&vistas, std::process::id(), &titulo) else {
        return Err(format!(
            "Wayfire no tiene ninguna vista «{titulo}» de este proceso"
        ));
    };

    send_request(
        &mut stream,
        "window-rules/focus-view",
        serde_json::json!({ "id": id }),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn vistas() -> Value {
        serde_json::json!([
            { "id": 12, "pid": 100, "title": "Otra aplicación" },
            { "id": 34, "pid": 527, "title": "MiniPlayer - Resonance" },
            { "id": 56, "pid": 527, "title": "Resonance" },
            { "id": 78, "pid": 999, "title": "Resonance" }
        ])
    }

    #[test]
    fn encuentra_la_ventana_por_pid_y_titulo() {
        assert_eq!(vista_del_proceso(&vistas(), 527, "Resonance"), Some(56));
    }

    /// Las dos ventanas del reproductor llevan el mismo pid, y el
    /// mini-reproductor viene **antes** en la lista: buscar sólo por pid
    /// devolvería ése.
    #[test]
    fn no_confunde_la_principal_con_el_mini_reproductor() {
        assert_eq!(
            vista_del_proceso(&vistas(), 527, "MiniPlayer - Resonance"),
            Some(34)
        );
    }

    /// Y otra instancia con el mismo título es de otro proceso.
    #[test]
    fn no_agarra_la_ventana_de_otra_instancia() {
        assert_ne!(vista_del_proceso(&vistas(), 527, "Resonance"), Some(78));
    }

    #[test]
    fn sin_esa_ventana_no_devuelve_nada() {
        assert_eq!(vista_del_proceso(&vistas(), 527, "Inexistente"), None);
        assert_eq!(
            vista_del_proceso(&serde_json::json!({}), 527, "Resonance"),
            None
        );
    }
}
