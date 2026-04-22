use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

use crate::structs::Track;

pub fn get_database_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "No se pudo resolver HOME".to_string())?;

    Ok(home.join(".config/vasak/resonance.db"))
}

pub fn open_database(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear ~/.config/vasak: {e}"))?;
    }

    let conn = Connection::open(db_path)
        .map_err(|e| format!("No se pudo abrir la base de datos SQLite: {e}"))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
        ",
    )
    .map_err(|e| format!("No se pudo inicializar el esquema SQLite: {e}"))?;

    Ok(conn)
}

pub fn insert_track_if_not_exists(conn: &Connection, track: &Track) -> Result<bool, String> {
    let affected = conn
        .execute(
            "
            INSERT OR IGNORE INTO tracks (path, title, artist, album, duration_seconds)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                track.path,
                track.title,
                track.artist,
                track.album,
                track.duration_seconds
            ],
        )
        .map_err(|e| format!("No se pudo insertar track en SQLite: {e}"))?;

    Ok(affected > 0)
}
