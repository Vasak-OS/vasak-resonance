use crate::db::{get_database_path, list_tracks, open_database, search_tracks_fts, upsert_track};
use crate::structs::LibraryTrack;
use crate::structs::Track;

/// Runs a database operation off the main thread.
///
/// A synchronous `#[tauri::command]` executes on the main thread, so every
/// library query froze the interface while SQLite worked — most visibly on
/// search, which runs on each keystroke over the whole library.
async fn with_database<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&rusqlite::Connection) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let db_path = get_database_path()?;
        let conn = open_database(&db_path)?;
        work(&conn)
    })
    .await
    .map_err(|e| format!("La consulta a la biblioteca falló: {e}"))?
}

#[tauri::command]
pub async fn list_library_tracks() -> Result<Vec<LibraryTrack>, String> {
    with_database(|conn| list_tracks(conn)).await
}

#[tauri::command]
pub async fn save_library_track(track: Track) -> Result<(), String> {
    with_database(move |conn| upsert_track(conn, &track)).await
}

#[tauri::command]
pub async fn search_library_tracks(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<LibraryTrack>, String> {
    with_database(move |conn| search_tracks_fts(conn, &query, limit.unwrap_or(2000))).await
}
