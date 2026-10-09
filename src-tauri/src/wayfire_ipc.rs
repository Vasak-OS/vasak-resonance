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

    if let Ok(display) = env::var("WAYLAND_DISPLAY") {
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
pub fn find_process_view(views: &Value, pid: u32, title: &str) -> Option<i64> {
    views
        .as_array()?
        .iter()
        .find(|view| {
            view.get("pid").and_then(Value::as_u64) == Some(u64::from(pid))
                && view.get("title").and_then(Value::as_str) == Some(title)
        })
        .and_then(|view| view.get("id").and_then(Value::as_i64))
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
pub async fn focus_own_window(title: String) -> Result<(), String> {
    if find_socket().is_none() {
        return Ok(());
    }

    let mut stream = connect().await?;
    let views = send_request(
        &mut stream,
        "window-rules/list-views",
        serde_json::json!({}),
    )
    .await?;

    let Some(id) = find_process_view(&views, std::process::id(), &title) else {
        return Err(format!(
            "Wayfire no tiene ninguna vista «{title}» de este proceso"
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

/// Dónde va una ventana de `size` en la esquina de abajo a la derecha de
/// `workarea`, a `margin` de los dos bordes.
///
/// `workarea` es lo que devuelve `window-rules/output-info`: la pantalla menos
/// lo que reservan el panel y las demás capas, en coordenadas de la pantalla.
/// Sin las cuatro medidas no hay geometría.
pub fn bottom_right_geometry(workarea: &Value, size: (i32, i32), margin: i32) -> Option<Value> {
    let measure = |key: &str| workarea.get(key).and_then(Value::as_f64);
    let (x, y) = (measure("x")?, measure("y")?);
    let (width, height) = (measure("width")?, measure("height")?);
    let (view_width, view_height) = (f64::from(size.0), f64::from(size.1));
    let margin = f64::from(margin);

    Some(serde_json::json!({
        "x": (x + width - view_width - margin).max(x).round() as i64,
        "y": (y + height - view_height - margin).max(y).round() as i64,
        "width": size.0,
        "height": size.1,
    }))
}

/// Pone una ventana de este proceso abajo a la derecha y encima de las demás.
///
/// Es la reserva para cuando el compositor no ofrece `wlr-layer-shell`: una
/// ventana común no puede elegir dónde va, pero a Wayfire sí se le puede pedir
/// que la mueva. Se llama **cada vez** que se muestra, porque al volver a
/// mapearla el compositor la acomoda de nuevo a su criterio.
///
/// La vista aparece en la lista recién cuando el compositor la mapea, que es un
/// poco después del `show()`: por eso se la espera un rato. Fuera de Wayfire no
/// hay socket, y entonces no hace nada.
pub async fn place_own_view_bottom_right(
    title: &str,
    size: (i32, i32),
    margin: i32,
) -> Result<(), String> {
    if find_socket().is_none() {
        return Ok(());
    }

    let mut stream = connect().await?;

    let mut found = None;
    for _ in 0..40 {
        let views = send_request(
            &mut stream,
            "window-rules/list-views",
            serde_json::json!({}),
        )
        .await?;
        found = mapped_process_view(&views, std::process::id(), title);
        if found.is_some() {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    let Some((id, output_id)) = found else {
        return Err(format!(
            "Wayfire no mapeó ninguna vista «{title}» de este proceso"
        ));
    };

    let output = send_request(
        &mut stream,
        "window-rules/output-info",
        serde_json::json!({ "id": output_id }),
    )
    .await?;
    let geometry = output
        .get("workarea")
        .and_then(|workarea| bottom_right_geometry(workarea, size, margin))
        .ok_or("Wayfire no dijo el área útil de la pantalla")?;

    send_request(
        &mut stream,
        "window-rules/configure-view",
        serde_json::json!({ "id": id, "output_id": output_id, "geometry": geometry }),
    )
    .await?;

    // `alwaysOnTop` de Tauri no llega a Wayland. Se pide acá, y si el
    // complemento `wm-actions` no está cargado la ventana queda bien puesta
    // igual: no es motivo para avisar de un error.
    let _ = send_request(
        &mut stream,
        "wm-actions/set-always-on-top",
        serde_json::json!({ "view_id": id, "state": true }),
    )
    .await;

    Ok(())
}

/// Como [`find_process_view`], pero sólo si ya está mapeada, y con la pantalla
/// en la que está.
fn mapped_process_view(views: &Value, pid: u32, title: &str) -> Option<(i64, i64)> {
    let id = find_process_view(views, pid, title)?;
    let view = views
        .as_array()?
        .iter()
        .find(|view| view.get("id").and_then(Value::as_i64) == Some(id))?;

    if view.get("mapped").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let output_id = view.get("output-id").and_then(Value::as_i64)?;
    Some((id, output_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_esquina_de_abajo_a_la_derecha_deja_el_margen() {
        let workarea = serde_json::json!({ "x": 0.0, "y": 0.0, "width": 1920.0, "height": 1080.0 });
        assert_eq!(
            bottom_right_geometry(&workarea, (360, 120), 10),
            Some(serde_json::json!({ "x": 1550, "y": 950, "width": 360, "height": 120 }))
        );
    }

    /// El área útil ya descuenta el panel: con un panel de 40 px abajo, el
    /// mini reproductor queda encima de él y no tapado.
    #[test]
    fn respeta_lo_que_reserva_el_panel() {
        let workarea = serde_json::json!({ "x": 0.0, "y": 0.0, "width": 1920.0, "height": 1040.0 });
        let geometry = bottom_right_geometry(&workarea, (360, 120), 10).unwrap();
        assert_eq!(geometry["y"], 910);

        let workarea =
            serde_json::json!({ "x": 48.0, "y": 0.0, "width": 1872.0, "height": 1080.0 });
        let geometry = bottom_right_geometry(&workarea, (360, 120), 10).unwrap();
        assert_eq!(geometry["x"], 1550);
    }

    /// En una pantalla más chica que la ventana, mejor recortada por abajo y a
    /// la derecha que con la esquina de arriba fuera de la pantalla.
    #[test]
    fn no_se_va_por_arriba_ni_por_la_izquierda() {
        let workarea = serde_json::json!({ "x": 0.0, "y": 0.0, "width": 300.0, "height": 100.0 });
        let geometry = bottom_right_geometry(&workarea, (360, 120), 10).unwrap();
        assert_eq!(
            (geometry["x"].as_i64(), geometry["y"].as_i64()),
            (Some(0), Some(0))
        );
    }

    #[test]
    fn sin_las_cuatro_medidas_no_hay_geometria() {
        let workarea = serde_json::json!({ "x": 0.0, "y": 0.0, "width": 1920.0 });
        assert_eq!(bottom_right_geometry(&workarea, (360, 120), 10), None);
    }

    #[test]
    fn espera_a_que_la_vista_este_mapeada() {
        let views = serde_json::json!([
            { "id": 34, "pid": 527, "title": "MiniPlayer - Resonance", "mapped": false, "output-id": 2 }
        ]);
        assert_eq!(
            mapped_process_view(&views, 527, "MiniPlayer - Resonance"),
            None
        );

        let views = serde_json::json!([
            { "id": 34, "pid": 527, "title": "MiniPlayer - Resonance", "mapped": true, "output-id": 2 }
        ]);
        assert_eq!(
            mapped_process_view(&views, 527, "MiniPlayer - Resonance"),
            Some((34, 2))
        );
    }

    fn views() -> Value {
        serde_json::json!([
            { "id": 12, "pid": 100, "title": "Otra aplicación" },
            { "id": 34, "pid": 527, "title": "MiniPlayer - Resonance" },
            { "id": 56, "pid": 527, "title": "Resonance" },
            { "id": 78, "pid": 999, "title": "Resonance" }
        ])
    }

    #[test]
    fn encuentra_la_ventana_por_pid_y_titulo() {
        assert_eq!(find_process_view(&views(), 527, "Resonance"), Some(56));
    }

    /// Las dos ventanas del reproductor llevan el mismo pid, y el
    /// mini-reproductor viene **antes** en la lista: buscar sólo por pid
    /// devolvería ése.
    #[test]
    fn no_confunde_la_principal_con_el_mini_reproductor() {
        assert_eq!(
            find_process_view(&views(), 527, "MiniPlayer - Resonance"),
            Some(34)
        );
    }

    /// Y otra instancia con el mismo título es de otro proceso.
    #[test]
    fn no_agarra_la_ventana_de_otra_instancia() {
        assert_ne!(find_process_view(&views(), 527, "Resonance"), Some(78));
    }

    #[test]
    fn sin_esa_ventana_no_devuelve_nada() {
        assert_eq!(find_process_view(&views(), 527, "Inexistente"), None);
        assert_eq!(
            find_process_view(&serde_json::json!({}), 527, "Resonance"),
            None
        );
    }
}
