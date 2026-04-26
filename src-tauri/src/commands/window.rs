use tauri::{AppHandle, Manager, PhysicalPosition, Position};

fn position_miniplayer_bottom_right(app: &AppHandle) -> Result<(), String> {
    let mini_window = app
        .get_webview_window("mini-player")
        .ok_or_else(|| "No se encontro la ventana MiniPlayer".to_string())?;

    let monitor = mini_window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No se encontro monitor activo".to_string())?;

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let window_size = mini_window.outer_size().map_err(|error| error.to_string())?;

    let margin = 18i32;
    let target_x = monitor_position.x + monitor_size.width as i32 - window_size.width as i32 - margin;
    let target_y = monitor_position.y + monitor_size.height as i32 - window_size.height as i32 - margin;

    mini_window
        .set_position(Position::Physical(PhysicalPosition::new(target_x, target_y)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_main_and_miniplayer(app: AppHandle) -> Result<(), String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "No se encontro la ventana principal".to_string())?;
    let mini_window = app
        .get_webview_window("mini-player")
        .ok_or_else(|| "No se encontro la ventana MiniPlayer".to_string())?;

    let mini_visible = mini_window.is_visible().map_err(|error| error.to_string())?;

    if mini_visible {
        mini_window.hide().map_err(|error| error.to_string())?;
        main_window.show().map_err(|error| error.to_string())?;
        main_window.set_focus().map_err(|error| error.to_string())?;
    } else {
        main_window.hide().map_err(|error| error.to_string())?;
        mini_window.show().map_err(|error| error.to_string())?;
        position_miniplayer_bottom_right(&app)?;
        mini_window.set_focus().map_err(|error| error.to_string())?;
    }

    Ok(())
}
