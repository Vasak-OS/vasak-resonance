use tauri::{AppHandle, Manager};

#[cfg(target_os = "linux")]
use crate::layer_shell;

/// Pasa de la ventana principal al mini reproductor y al revés.
///
/// Todo lo que toca ventanas va en el hilo principal: el mini reproductor puede
/// ser una `gtk::Window` de capa (ver `layer_shell`), y GTK sólo se puede tocar
/// desde ahí. Este comando es asíncrono y corre en otro hilo.
#[tauri::command]
pub async fn toggle_main_and_miniplayer(app: AppHandle) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(swap_windows(&handle));
    })
    .map_err(|error| error.to_string())?;

    let mini_shown_as_toplevel = receiver.await.map_err(|error| error.to_string())??;

    // Sin capa, el mini reproductor es una ventana común y el compositor la
    // pone donde quiere: se le pide a Wayfire que la lleve a la esquina. Si no
    // se puede, queda donde la puso el compositor, que es como estaba antes.
    #[cfg(target_os = "linux")]
    if mini_shown_as_toplevel {
        if let Err(error) = crate::wayfire_ipc::place_own_view_bottom_right(
            layer_shell::MINI_PLAYER_TITLE,
            layer_shell::MINI_PLAYER_SIZE,
            layer_shell::MINI_PLAYER_MARGIN,
        )
        .await
        {
            eprintln!("[layer_shell] no se pudo acomodar el mini reproductor: {error}");
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = mini_shown_as_toplevel;

    Ok(())
}

/// Esconde una y muestra la otra. Devuelve `true` si acaba de mostrar el mini
/// reproductor como ventana común, que es cuando hay que acomodarlo.
fn swap_windows(app: &AppHandle) -> Result<bool, String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "No se encontro la ventana principal".to_string())?;
    let mini_window = app
        .get_webview_window("mini-player")
        .ok_or_else(|| "No se encontro la ventana MiniPlayer".to_string())?;

    #[cfg(target_os = "linux")]
    let layer_visible = layer_shell::layer_visible();
    #[cfg(not(target_os = "linux"))]
    let layer_visible: Option<bool> = None;

    let mini_visible = match layer_visible {
        Some(visible) => visible,
        None => mini_window
            .is_visible()
            .map_err(|error| error.to_string())?,
    };

    if mini_visible {
        set_mini_visible(&mini_window, false)?;
        main_window.show().map_err(|error| error.to_string())?;
        main_window.set_focus().map_err(|error| error.to_string())?;
        Ok(false)
    } else {
        let as_toplevel = set_mini_visible(&mini_window, true)?;
        main_window.hide().map_err(|error| error.to_string())?;
        if as_toplevel {
            mini_window.set_focus().map_err(|error| error.to_string())?;
        }
        Ok(as_toplevel)
    }
}

/// Muestra o esconde el mini reproductor, sea capa o ventana común. Devuelve
/// `true` si lo hizo con la ventana de Tauri.
fn set_mini_visible(mini_window: &tauri::WebviewWindow, visible: bool) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    if layer_shell::set_layer_visible(visible) {
        return Ok(false);
    }

    if visible {
        mini_window.show().map_err(|error| error.to_string())?;
    } else {
        mini_window.hide().map_err(|error| error.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
pub fn close_app(app: AppHandle) -> Result<(), String> {
    use crate::audio_manager::AudioState;

    if let Some(audio_state) = app.try_state::<AudioState>() {
        let _ = audio_state.shutdown();
    }

    std::process::exit(0);
}
