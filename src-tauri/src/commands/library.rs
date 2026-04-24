use crate::db::{get_database_path, list_tracks, open_database, upsert_track};
use crate::structs::LibraryTrack;
use crate::structs::Track;

#[tauri::command]
pub fn list_library_tracks() -> Result<Vec<LibraryTrack>, String> {
	let db_path = get_database_path()?;
	let conn = open_database(&db_path)?;
	list_tracks(&conn)
}

#[tauri::command]
pub fn save_library_track(track: Track) -> Result<(), String> {
	let db_path = get_database_path()?;
	let conn = open_database(&db_path)?;
	upsert_track(&conn, &track)
}
