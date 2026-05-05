use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

use crate::structs::{LibraryTrack, Playlist, PlaylistTrack, Track};

const NEW_DB_RELATIVE_PATH: &str = ".config/resonance/resonance.db";
const LEGACY_DB_RELATIVE_PATH: &str = ".config/vasak/resonance.db";

fn resolve_home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "No se pudo resolver HOME".to_string())
}

pub fn get_database_path() -> Result<PathBuf, String> {
    let home = resolve_home_dir()?;
    let new_path = home.join(NEW_DB_RELATIVE_PATH);
    let legacy_path = home.join(LEGACY_DB_RELATIVE_PATH);

    // Migra automáticamente la base legada al nuevo directorio de config.
    if !new_path.exists() && legacy_path.exists() {
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear ~/.config/resonance: {e}"))?;
        }

        fs::rename(&legacy_path, &new_path).map_err(|e| {
            format!(
                "No se pudo migrar la base de datos desde {} a {}: {e}",
                legacy_path.display(),
                new_path.display()
            )
        })?;
    }

    Ok(new_path)
}

pub fn open_database(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear ~/.config/resonance: {e}"))?;
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

        CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
            title,
            artist,
            album,
            path UNINDEXED,
            content='tracks',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
            INSERT INTO tracks_fts(rowid, title, artist, album, path)
            VALUES (new.id, new.title, new.artist, new.album, new.path);
        END;

        CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
            INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, path)
            VALUES ('delete', old.id, old.title, old.artist, old.album, old.path);
        END;

        CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
            INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, path)
            VALUES ('delete', old.id, old.title, old.artist, old.album, old.path);
            INSERT INTO tracks_fts(rowid, title, artist, album, path)
            VALUES (new.id, new.title, new.artist, new.album, new.path);
        END;
        ",
    )
    .map_err(|e| format!("No se pudo inicializar el esquema SQLite: {e}"))?;

    conn.execute("INSERT INTO tracks_fts(tracks_fts) VALUES ('rebuild')", [])
        .map_err(|e| format!("No se pudo reconstruir índice FTS5: {e}"))?;

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

pub fn upsert_track(conn: &Connection, track: &Track) -> Result<(), String> {
    conn.execute(
        "
        INSERT INTO tracks (path, title, artist, album, duration_seconds)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist = excluded.artist,
            album = excluded.album,
            duration_seconds = excluded.duration_seconds
        ",
        params![
            track.path,
            track.title,
            track.artist,
            track.album,
            track.duration_seconds
        ],
    )
    .map_err(|e| format!("No se pudo sincronizar track en SQLite: {e}"))?;

    Ok(())
}

pub fn list_tracks(conn: &Connection) -> Result<Vec<LibraryTrack>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT id, path, title, artist, album, duration_seconds, created_at
            FROM tracks
            ORDER BY created_at DESC, title COLLATE NOCASE ASC
            ",
        )
        .map_err(|e| format!("No se pudo preparar query de tracks: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(LibraryTrack {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_seconds: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("No se pudo consultar tracks: {e}"))?;

    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(row.map_err(|e| format!("No se pudo leer track: {e}"))?);
    }

    Ok(tracks)
}

pub fn search_tracks_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<LibraryTrack>, String> {
    let fts_query = build_fts_query(query);
    if fts_query.is_empty() {
        return list_tracks(conn);
    }

    let clamped_limit = limit.clamp(1, 10_000) as i64;

    let mut stmt = conn
        .prepare(
            "
            SELECT
                t.id,
                t.path,
                t.title,
                t.artist,
                t.album,
                t.duration_seconds,
                t.created_at
            FROM tracks_fts f
            JOIN tracks t ON t.id = f.rowid
            WHERE tracks_fts MATCH ?1
            ORDER BY bm25(tracks_fts, 1.2, 1.0, 0.9), t.created_at DESC
            LIMIT ?2
            ",
        )
        .map_err(|e| format!("No se pudo preparar búsqueda FTS5: {e}"))?;

    let rows = stmt
        .query_map(params![fts_query, clamped_limit], |row| {
            Ok(LibraryTrack {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_seconds: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("No se pudo ejecutar búsqueda FTS5: {e}"))?;

    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(row.map_err(|e| format!("No se pudo leer resultado FTS5: {e}"))?);
    }

    if !tracks.is_empty() {
        return Ok(tracks);
    }

    search_tracks_contains(conn, query, limit)
}

fn search_tracks_contains(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<LibraryTrack>, String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect();

    if tokens.is_empty() {
        return list_tracks(conn);
    }

    let clamped_limit = limit.clamp(1, 10_000) as i64;

    let mut sql = String::from(
        "
        SELECT id, path, title, artist, album, duration_seconds, created_at
        FROM tracks
        WHERE 1 = 1
        ",
    );

    for _ in &tokens {
        sql.push_str(
            "
            AND (
                LOWER(title) LIKE ?
                OR LOWER(artist) LIKE ?
                OR LOWER(album) LIKE ?
            )
            ",
        );
    }

    sql.push_str(
        "
        ORDER BY created_at DESC, title COLLATE NOCASE ASC
        LIMIT ?
        ",
    );

    let mut params: Vec<String> = Vec::with_capacity(tokens.len() * 3 + 1);
    for token in &tokens {
        let wildcard = format!("%{}%", token);
        params.push(wildcard.clone());
        params.push(wildcard.clone());
        params.push(wildcard);
    }
    params.push(clamped_limit.to_string());

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("No se pudo preparar búsqueda contains: {e}"))?;

    let rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(LibraryTrack {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                artist: row.get(3)?,
                album: row.get(4)?,
                duration_seconds: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("No se pudo ejecutar búsqueda contains: {e}"))?;

    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(row.map_err(|e| format!("No se pudo leer resultado contains: {e}"))?);
    }

    Ok(tracks)
}

fn build_fts_query(raw: &str) -> String {
    raw.split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"*", token.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" AND ")
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

pub fn list_playlist_tracks(
    conn: &Connection,
    playlist_id: i64,
) -> Result<Vec<PlaylistTrack>, String> {
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
