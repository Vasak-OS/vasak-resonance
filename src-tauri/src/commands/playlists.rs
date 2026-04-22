use crate::db::{
    add_track_to_playlist, create_playlist, delete_playlist, get_database_path, list_playlist_tracks,
    list_playlists, open_database, remove_track_from_playlist,
};
use crate::structs::{Playlist, PlaylistTrack};

#[tauri::command]
pub fn create_playlist_command(name: String) -> Result<Playlist, String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    create_playlist(&conn, &name)
}

#[tauri::command]
pub fn list_playlists_command() -> Result<Vec<Playlist>, String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    list_playlists(&conn)
}

#[tauri::command]
pub fn delete_playlist_command(playlist_id: i64) -> Result<(), String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    delete_playlist(&conn, playlist_id)
}

#[tauri::command]
pub fn add_track_to_playlist_command(playlist_id: i64, track_id: i64) -> Result<(), String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    add_track_to_playlist(&conn, playlist_id, track_id)
}

#[tauri::command]
pub fn remove_track_from_playlist_command(playlist_id: i64, track_id: i64) -> Result<(), String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    remove_track_from_playlist(&conn, playlist_id, track_id)
}

#[tauri::command]
pub fn list_playlist_tracks_command(playlist_id: i64) -> Result<Vec<PlaylistTrack>, String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;
    list_playlist_tracks(&conn, playlist_id)
}
