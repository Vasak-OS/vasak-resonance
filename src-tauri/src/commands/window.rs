use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn toggle_main_and_miniplayer(app: AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "No se encontro la ventana principal".to_string())?;
    let mini_window = app
        .get_webview_window("mini-player")
        .ok_or_else(|| "No se encontro la ventana MiniPlayer".to_string())?;

    let mini_visible = mini_window
        .is_visible()
        .map_err(|error| error.to_string())?;

    if mini_visible {
        mini_window.hide().map_err(|error| error.to_string())?;
        main_window.show().map_err(|error| error.to_string())?;
        main_window.set_focus().map_err(|error| error.to_string())?;
    } else {
        mini_window.show().map_err(|error| error.to_string())?;
        main_window.hide().map_err(|error| error.to_string())?;
        mini_window.set_focus().map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn close_app(app: AppHandle) -> Result<(), String> {
    use crate::audio_manager::AudioState;

    if let Some(audio_state) = app.try_state::<AudioState>() {
        let _ = audio_state.shutdown();
    }

    std::process::exit(0);
}
