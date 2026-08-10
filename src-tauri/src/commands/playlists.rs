use crate::db::{
    add_track_to_playlist, create_playlist, delete_playlist, get_database_path,
    list_playlist_tracks, list_playlists, open_database, remove_track_from_playlist,
};
use crate::structs::{Playlist, PlaylistTrack};

/// Same reasoning as the library commands: SQLite work belongs off the main
/// thread, or the window stops repainting while it runs.
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
    .map_err(|e| format!("La consulta de listas falló: {e}"))?
}

#[tauri::command]
pub async fn create_playlist_command(name: String) -> Result<Playlist, String> {
    with_database(move |conn| create_playlist(conn, &name)).await
}

#[tauri::command]
pub async fn list_playlists_command() -> Result<Vec<Playlist>, String> {
    with_database(list_playlists).await
}

#[tauri::command]
pub async fn delete_playlist_command(playlist_id: i64) -> Result<(), String> {
    with_database(move |conn| delete_playlist(conn, playlist_id)).await
}

#[tauri::command]
pub async fn add_track_to_playlist_command(playlist_id: i64, track_id: i64) -> Result<(), String> {
    with_database(move |conn| add_track_to_playlist(conn, playlist_id, track_id)).await
}

#[tauri::command]
pub async fn remove_track_from_playlist_command(
    playlist_id: i64,
    track_id: i64,
) -> Result<(), String> {
    with_database(move |conn| remove_track_from_playlist(conn, playlist_id, track_id)).await
}

#[tauri::command]
pub async fn list_playlist_tracks_command(playlist_id: i64) -> Result<Vec<PlaylistTrack>, String> {
    with_database(move |conn| list_playlist_tracks(conn, playlist_id)).await
}
