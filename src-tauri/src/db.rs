use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

use crate::structs::{Playlist, PlaylistTrack, Track};

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

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("No se pudo habilitar foreign_keys en SQLite: {e}"))?;

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

        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER NOT NULL,
            track_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (playlist_id, track_id),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist_position
            ON playlist_tracks(playlist_id, position);
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

pub fn create_playlist(conn: &Connection, name: &str) -> Result<Playlist, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("El nombre de la playlist no puede estar vacío".to_string());
    }

    conn.execute(
        "INSERT INTO playlists (name) VALUES (?1)",
        params![trimmed_name],
    )
    .map_err(|e| format!("No se pudo crear playlist: {e}"))?;

    let playlist_id = conn.last_insert_rowid();
    get_playlist_by_id(conn, playlist_id)
}

pub fn list_playlists(conn: &Connection) -> Result<Vec<Playlist>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT id, name, created_at
            FROM playlists
            ORDER BY name COLLATE NOCASE ASC
            ",
        )
        .map_err(|e| format!("No se pudo preparar query de playlists: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| format!("No se pudo consultar playlists: {e}"))?;

    let mut playlists = Vec::new();
    for row in rows {
        playlists.push(row.map_err(|e| format!("No se pudo leer playlist: {e}"))?);
    }

    Ok(playlists)
}

pub fn delete_playlist(conn: &Connection, playlist_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])
        .map_err(|e| format!("No se pudo eliminar playlist: {e}"))?;
    Ok(())
}

pub fn add_track_to_playlist(
    conn: &Connection,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), String> {
    let next_position: i64 = conn
        .query_row(
            "
            SELECT COALESCE(MAX(position), -1) + 1
            FROM playlist_tracks
            WHERE playlist_id = ?1
            ",
            params![playlist_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("No se pudo calcular posición de playlist: {e}"))?;

    conn.execute(
        "
        INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
        VALUES (?1, ?2, ?3)
        ",
        params![playlist_id, track_id, next_position],
    )
    .map_err(|e| format!("No se pudo agregar track a playlist: {e}"))?;

    Ok(())
}

pub fn remove_track_from_playlist(
    conn: &Connection,
    playlist_id: i64,
    track_id: i64,
) -> Result<(), String> {
    conn.execute(
        "
        DELETE FROM playlist_tracks
        WHERE playlist_id = ?1 AND track_id = ?2
        ",
        params![playlist_id, track_id],
    )
    .map_err(|e| format!("No se pudo quitar track de playlist: {e}"))?;

    Ok(())
}

pub fn list_playlist_tracks(conn: &Connection, playlist_id: i64) -> Result<Vec<PlaylistTrack>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT
                pt.playlist_id,
                t.id,
                pt.position,
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration_seconds
            FROM playlist_tracks pt
            JOIN tracks t ON t.id = pt.track_id
            WHERE pt.playlist_id = ?1
            ORDER BY pt.position ASC
            ",
        )
        .map_err(|e| format!("No se pudo preparar query de tracks de playlist: {e}"))?;

    let rows = stmt
        .query_map(params![playlist_id], |row| {
            Ok(PlaylistTrack {
                playlist_id: row.get(0)?,
                track_id: row.get(1)?,
                position: row.get(2)?,
                path: row.get(3)?,
                title: row.get(4)?,
                artist: row.get(5)?,
                album: row.get(6)?,
                duration_seconds: row.get(7)?,
            })
        })
        .map_err(|e| format!("No se pudo consultar tracks de playlist: {e}"))?;

    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(row.map_err(|e| format!("No se pudo leer track de playlist: {e}"))?);
    }

    Ok(tracks)
}

fn get_playlist_by_id(conn: &Connection, playlist_id: i64) -> Result<Playlist, String> {
    conn.query_row(
        "
        SELECT id, name, created_at
        FROM playlists
        WHERE id = ?1
        ",
        params![playlist_id],
        |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
    )
    .map_err(|e| format!("No se pudo obtener playlist creada: {e}"))
}
