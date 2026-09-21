use rusqlite::{params, Connection};
use std::collections::HashMap;
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

/// Databases whose schema this process has already set up.
///
/// Creating the tables and reindexing used to happen on *every* connection, and
/// a connection is opened per command — so a search reindexed the whole library
/// on each keystroke. The schema lives on disk; once per database is enough.
fn initialised_databases() -> &'static std::sync::Mutex<std::collections::HashSet<PathBuf>> {
    static PATHS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> =
        std::sync::OnceLock::new();
    PATHS.get_or_init(Default::default)
}

pub fn open_database(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear ~/.config/resonance: {e}"))?;
    }

    let conn = Connection::open(db_path)
        .map_err(|e| format!("No se pudo abrir la base de datos SQLite: {e}"))?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        -- Write-ahead logging so a scan writing tracks does not block the
        -- searches and listings the interface is doing at the same time.
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ",
    )
    .map_err(|e| format!("No se pudieron aplicar los PRAGMA de SQLite: {e}"))?;

    let mut initialised = initialised_databases()
        .lock()
        .map_err(|_| "No se pudo comprobar el estado del esquema".to_string())?;

    if !initialised.contains(db_path) {
        ensure_schema(&conn)?;
        initialised.insert(db_path.to_path_buf());
    }

    Ok(conn)
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            -- El artista del álbum: lo que mantiene junta una recopilación.
            -- Vacío cuando el archivo no lo dice.
            album_artist TEXT NOT NULL DEFAULT '',
            -- El número de pista, o 0 si el archivo no lo dice.
            track_no INTEGER NOT NULL DEFAULT 0,
            duration_seconds INTEGER NOT NULL,
            -- Cuándo se modificó el archivo por última vez, en segundos desde
            -- la época. Es lo que hace que un barrido no tenga que abrir un
            -- archivo que no cambió — y lo que hace que uno editado se vea.
            -- Cero es «no se sabe»: una fila de antes de que esto existiera.
            mtime INTEGER NOT NULL DEFAULT 0,
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

        -- Una fila por reproducción. Sólo la ruta y el momento: el título, el
        -- artista y el álbum están en `tracks`, y copiarlos acá los dejaría
        -- congelados el día que alguien corrija una etiqueta.
        --
        -- Sin clave foránea a propósito. Un archivo que se borra o se mueve no
        -- tiene por qué llevarse puesto el registro de que se lo escuchó, y una
        -- biblioteca rebarrida no es una biblioteca escuchada de nuevo.
        CREATE TABLE IF NOT EXISTS plays (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            played_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_plays_played_at ON plays(played_at);

        -- Cuatro cosas que la biblioteca necesita recordar de sí misma. Hoy
        -- sólo la versión de barrido; es una tabla y no un archivo para que
        -- viaje con la base y no pueda quedar desfasada de ella.
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_plays_path ON plays(path);

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

    // The triggers above keep the index current, so a rebuild is only needed
    // for a library indexed before they existed. Rebuilding reads every track
    // in the database, which is why it must not be routine.
    //
    // The count has to come from `tracks_fts_docsize`, the index's own storage.
    // Selecting from `tracks_fts` reads the *content* table it mirrors, so it
    // reports rows even when nothing has been indexed at all.
    let needs_rebuild: bool = conn
        .query_row(
            "SELECT (SELECT count(*) FROM tracks) > 0
                AND (SELECT count(*) FROM tracks_fts_docsize) = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("No se pudo comprobar el índice FTS5: {e}"))?;

    if needs_rebuild {
        conn.execute("INSERT INTO tracks_fts(tracks_fts) VALUES ('rebuild')", [])
            .map_err(|e| format!("No se pudo reconstruir índice FTS5: {e}"))?;
    }

    agregar_columnas_que_falten(conn)?;

    Ok(())
}

/// Le suma a una biblioteca vieja las columnas que no tenía.
///
/// `CREATE TABLE IF NOT EXISTS` no toca una tabla que ya existe, así que sin
/// esto una biblioteca de antes se queda sin las columnas nuevas y cualquier
/// consulta que las nombre falla.
///
/// Quedan en su valor por omisión —artista del álbum vacío, pista 0—, que es lo
/// mismo que dice un archivo sin esas etiquetas. Las de verdad las trae el
/// próximo barrido, que ya lee las etiquetas de todos los archivos igual.
fn agregar_columnas_que_falten(conn: &Connection) -> Result<(), String> {
    let existentes: std::collections::HashSet<String> = conn
        .prepare("SELECT name FROM pragma_table_info('tracks')")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()
        })
        .map_err(|e| format!("No se pudo leer el esquema de tracks: {e}"))?;

    for (columna, definicion) in [
        ("album_artist", "TEXT NOT NULL DEFAULT ''"),
        ("track_no", "INTEGER NOT NULL DEFAULT 0"),
        ("mtime", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if existentes.contains(columna) {
            continue;
        }

        conn.execute(
            &format!("ALTER TABLE tracks ADD COLUMN {columna} {definicion}"),
            [],
        )
        .map_err(|e| format!("No se pudo agregar la columna {columna}: {e}"))?;
    }

    Ok(())
}

/// Mete el tema, o lo actualiza entero si ya estaba.
///
/// No devuelve cuál de las dos cosas hizo: quien llama ya lo sabe —viene de
/// mirar [`known_mtimes_under`]— y preguntárselo a SQLite de nuevo sería pedir
/// dos veces lo mismo.
///
/// Entero y no dos columnas: quien llama acá es el barrido, y llega sólo cuando
/// el archivo es nuevo o cambió de fecha. Si cambió, lo que vale es lo que dice
/// el archivo ahora — que es lo que hace que corregir una etiqueta se vea.
///
/// El `mtime` se guarda con el resto: es la respuesta a «¿hace falta volver a
/// abrir este archivo?» la próxima vez.
pub fn index_track(conn: &Connection, track: &Track, mtime: i64) -> Result<(), String> {
    conn.execute(
        "
            INSERT INTO tracks
                (path, title, artist, album, album_artist, track_no, duration_seconds, mtime)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                album_artist = excluded.album_artist,
                track_no = excluded.track_no,
                duration_seconds = excluded.duration_seconds,
                mtime = excluded.mtime
            ",
        params![
            track.path,
            track.title,
            track.artist,
            track.album,
            track.album_artist,
            track.track_no,
            track.duration_seconds,
            mtime
        ],
    )
    .map_err(|e| format!("No se pudo indexar el track en SQLite: {e}"))?;

    Ok(())
}

/// Lo que la biblioteca ya sabe de los archivos que cuelgan de una carpeta:
/// ruta y fecha de modificación.
///
/// Una consulta por carpeta y no una por archivo. Con esto el barrido sabe, sin
/// volver a preguntar, qué archivos ya están, cuáles cambiaron y —lo que no se
/// puede saber archivo por archivo— cuáles están en la base y ya no en el disco.
pub fn known_mtimes_under(conn: &Connection, root: &str) -> Result<HashMap<String, i64>, String> {
    // `_` y `%` son comodines de `LIKE`: sin escaparlos, una carpeta que se
    // llame `mi_musica` alcanza también a `miXmusica`, y como lo que no se vio se
    // da de baja, eso borraría filas de una carpeta que nadie barrió. Los
    // guiones bajos en los nombres de carpeta son de lo más común.
    let prefijo = root
        .trim_end_matches('/')
        .replace('\\', "\\\\")
        .replace('_', "\\_")
        .replace('%', "\\%");
    let patron = format!("{prefijo}/%");

    let mut stmt = conn
        .prepare("SELECT path, mtime FROM tracks WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'")
        .map_err(|e| format!("No se pudo preparar la consulta de fechas: {e}"))?;

    let filas = stmt
        .query_map(params![root, patron], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("No se pudieron consultar las fechas: {e}"))?;

    let mut conocidos = HashMap::new();
    for fila in filas {
        let (path, mtime) = fila.map_err(|e| format!("No se pudo leer una fecha: {e}"))?;
        conocidos.insert(path, mtime);
    }

    Ok(conocidos)
}

/// Da de baja los temas que ya no están en el disco.
///
/// Las listas se limpian solas: `playlist_tracks` tiene la clave foránea con
/// `ON DELETE CASCADE` y los `PRAGMA` la activan. El historial de escuchas **no**
/// se toca, y es a propósito: que un archivo se borre no borra que sonó.
pub fn remove_tracks(conn: &mut Connection, paths: &[String]) -> Result<usize, String> {
    if paths.is_empty() {
        return Ok(0);
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("No se pudo abrir la transacción de baja: {e}"))?;

    let mut dados_de_baja = 0;
    {
        let mut stmt = tx
            .prepare("DELETE FROM tracks WHERE path = ?1")
            .map_err(|e| format!("No se pudo preparar la baja: {e}"))?;

        for path in paths {
            dados_de_baja += stmt
                .execute(params![path])
                .map_err(|e| format!("No se pudo dar de baja {path}: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("No se pudo confirmar la baja: {e}"))?;

    Ok(dados_de_baja)
}

/// Marca un tema como pendiente de releer, olvidando su fecha.
///
/// Se usa cuando el archivo está pero no se lo pudo leer: dejarle la fecha
/// guardada haría que el barrido siguiente lo diera por bueno sin abrirlo, y la
/// fila quedaría con lo que decía el archivo antes, para siempre.
pub fn forget_mtime(conn: &Connection, path: &str) -> Result<(), String> {
    conn.execute("UPDATE tracks SET mtime = 0 WHERE path = ?1", params![path])
        .map_err(|e| format!("No se pudo olvidar la fecha de {path}: {e}"))?;

    Ok(())
}

/// Lee un ajuste de la biblioteca.
pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

/// Guarda un ajuste de la biblioteca.
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("No se pudo guardar el ajuste {key}: {e}"))?;

    Ok(())
}

/// Guarda lo que la ventana sabe de un tema.
///
/// El artista del álbum y el número de pista **sólo se pisan con algo**: si lo
/// que llega viene vacío, se deja lo que había. Acá no llega un archivo, llega
/// el caché de la ventana, y ese caché puede ser de una versión anterior —donde
/// esos dos campos no existían— y traerlos en su valor por omisión. Sin esta
/// condición, abrir la aplicación borraría lo que el barrido acababa de leer.
///
/// Vaciarlos de verdad le toca al barrido, que es el que mira el archivo.
pub fn upsert_track(conn: &Connection, track: &Track) -> Result<(), String> {
    conn.execute(
        "
        INSERT INTO tracks
            (path, title, artist, album, album_artist, track_no, duration_seconds)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist = excluded.artist,
            album = excluded.album,
            album_artist = CASE
                WHEN excluded.album_artist <> '' THEN excluded.album_artist
                ELSE tracks.album_artist
            END,
            track_no = CASE
                WHEN excluded.track_no <> 0 THEN excluded.track_no
                ELSE tracks.track_no
            END,
            duration_seconds = excluded.duration_seconds
        ",
        params![
            track.path,
            track.title,
            track.artist,
            track.album,
            track.album_artist,
            track.track_no,
            track.duration_seconds
        ],
    )
    .map_err(|e| format!("No se pudo sincronizar track en SQLite: {e}"))?;

    Ok(())
}

/// Anota que un tema se escuchó, con el momento en segundos desde la época.
///
/// No hay «anotar una sola vez»: quién decide que una escucha cuenta es
/// `historial`, y acá cada llamada es una escucha más. Escuchar el mismo tema
/// tres veces son tres filas.
pub fn record_play(conn: &Connection, path: &str, played_at: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO plays (path, played_at) VALUES (?1, ?2)",
        rusqlite::params![path, played_at],
    )
    .map_err(|e| format!("No se pudo anotar la reproducción: {e}"))?;

    Ok(())
}

pub fn list_tracks(conn: &Connection) -> Result<Vec<LibraryTrack>, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT id, path, title, artist, album, album_artist, track_no,
                   duration_seconds, created_at
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
                album_artist: row.get(5)?,
                track_no: row.get(6)?,
                duration_seconds: row.get(7)?,
                created_at: row.get(8)?,
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
                t.album_artist,
                t.track_no,
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
                album_artist: row.get(5)?,
                track_no: row.get(6)?,
                duration_seconds: row.get(7)?,
                created_at: row.get(8)?,
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
        SELECT id, path, title, artist, album, album_artist, track_no,
               duration_seconds, created_at
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
                album_artist: row.get(5)?,
                track_no: row.get(6)?,
                duration_seconds: row.get(7)?,
                created_at: row.get(8)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn track(path: &str, title: &str, artist: &str, album: &str) -> Track {
        Track {
            id: None,
            path: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            album_artist: String::new(),
            track_no: 0,
            duration_seconds: 180,
        }
    }

    /// El mismo tema, pero de un disco: con su número de pista y su artista de
    /// álbum.
    fn pista(path: &str, title: &str, artist: &str, album_artist: &str, numero: i64) -> Track {
        Track {
            id: None,
            path: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: "Un álbum".to_string(),
            album_artist: album_artist.to_string(),
            track_no: numero,
            duration_seconds: 180,
        }
    }

    /// Mete el tema en la biblioteca, como lo haría un barrido.
    fn indexar(conn: &Connection, track: &Track) {
        index_track(conn, track, 0).expect("indexar");
    }

    fn temp_database() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("temp dir");
        let conn = open_database(&dir.path().join("resonance.db")).expect("open");
        (dir, conn)
    }

    #[test]
    fn a_new_track_is_searchable_immediately() {
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &track(
                "/m/a.mp3",
                "Bohemian Rhapsody",
                "Queen",
                "A Night at the Opera",
            ),
        );

        let by_title = search_tracks_fts(&conn, "bohemian", 10).expect("search");
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].title, "Bohemian Rhapsody");

        assert_eq!(
            search_tracks_fts(&conn, "queen", 10).expect("search").len(),
            1
        );
        assert!(search_tracks_fts(&conn, "nothingmatchesthis", 10)
            .expect("search")
            .is_empty());
    }

    /// The FTS triggers have to follow edits, or a renamed track keeps being
    /// found under its old title and not its new one.
    #[test]
    fn editing_a_track_updates_what_it_is_found_by() {
        let (_dir, conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "Old Title", "Artist", "Album"));

        upsert_track(&conn, &track("/m/a.mp3", "New Title", "Artist", "Album")).expect("upsert");

        assert!(search_tracks_fts(&conn, "old", 10)
            .expect("search")
            .is_empty());
        assert_eq!(
            search_tracks_fts(&conn, "new", 10).expect("search").len(),
            1
        );
        assert_eq!(
            list_tracks(&conn).expect("list").len(),
            1,
            "no duplicate row"
        );
    }

    #[test]
    fn el_disco_guarda_el_numero_de_pista_y_el_artista_del_album() {
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &pista("/m/03.mp3", "Tercera", "Alguien", "Varios", 3),
        );

        let guardado = list_tracks(&conn).expect("list").pop().expect("una fila");
        assert_eq!(guardado.track_no, 3);
        assert_eq!(guardado.album_artist, "Varios");
    }

    #[test]
    fn volver_a_indexar_un_tema_actualiza_la_fila_entera() {
        // El archivo cambió y el barrido lo relee: lo que vale es lo que dice
        // el archivo ahora. Esto es lo que hace que corregir una etiqueta se
        // vea, que antes no pasaba nunca.
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &track("/m/03.mp3", "Titulo viejo", "Alguien", "Un álbum"),
        );

        index_track(
            &conn,
            &pista("/m/03.mp3", "Título corregido", "Alguien", "Varios", 3),
            1_700_000_000,
        )
        .expect("segundo barrido");

        let guardado = list_tracks(&conn).expect("list").pop().expect("una fila");
        assert_eq!(guardado.title, "Título corregido");
        assert_eq!(guardado.track_no, 3);
        assert_eq!(guardado.album_artist, "Varios");
        assert_eq!(
            list_tracks(&conn).expect("list").len(),
            1,
            "sigue siendo una fila"
        );
    }

    #[test]
    fn la_fecha_del_archivo_queda_guardada() {
        let (_dir, conn) = temp_database();
        index_track(
            &conn,
            &track("/m/a.mp3", "Un tema", "Alguien", "Un álbum"),
            1_700_000_000,
        )
        .expect("indexar");

        let conocidos = known_mtimes_under(&conn, "/m").expect("consultar");
        assert_eq!(conocidos.get("/m/a.mp3"), Some(&1_700_000_000));
    }

    #[test]
    fn las_fechas_que_se_consultan_son_las_de_esa_carpeta() {
        // La baja de lo que ya no está se calcula con esto, así que traer de
        // más sería dar de baja temas de otra carpeta que nadie barrió.
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &track("/m/adentro.mp3", "Adentro", "Alguien", "Un álbum"),
        );
        indexar(
            &conn,
            &track("/otra/afuera.mp3", "Afuera", "Alguien", "Un álbum"),
        );
        // Una carpeta que empieza igual pero es otra.
        indexar(
            &conn,
            &track("/musica-vieja/x.mp3", "Otra", "Alguien", "Un álbum"),
        );

        let conocidos = known_mtimes_under(&conn, "/m").expect("consultar");

        assert_eq!(conocidos.len(), 1);
        assert!(conocidos.contains_key("/m/adentro.mp3"));
    }

    #[test]
    fn un_guion_bajo_en_el_nombre_de_la_carpeta_no_alcanza_a_otra() {
        // `_` es un comodín de `LIKE`. Sin escaparlo, barrer `/m/mi_musica`
        // habría traído las filas de `/m/miXmusica` y, como no se las ve al
        // recorrer, las habría dado de baja.
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &track("/m/mi_musica/a.mp3", "Propia", "Alguien", "Un álbum"),
        );
        indexar(
            &conn,
            &track("/m/miXmusica/b.mp3", "Ajena", "Alguien", "Un álbum"),
        );

        let conocidos = known_mtimes_under(&conn, "/m/mi_musica").expect("consultar");

        assert_eq!(conocidos.len(), 1);
        assert!(conocidos.contains_key("/m/mi_musica/a.mp3"));
    }

    #[test]
    fn un_porcentaje_en_el_nombre_tampoco() {
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &track("/m/100%/a.mp3", "Propia", "Alguien", "Un álbum"),
        );
        indexar(
            &conn,
            &track("/m/100 por ciento/b.mp3", "Ajena", "Alguien", "Un álbum"),
        );

        let conocidos = known_mtimes_under(&conn, "/m/100%").expect("consultar");

        assert_eq!(conocidos.len(), 1);
        assert!(conocidos.contains_key("/m/100%/a.mp3"));
    }

    #[test]
    fn olvidar_la_fecha_obliga_a_releer_el_archivo() {
        let (_dir, conn) = temp_database();
        index_track(
            &conn,
            &track("/m/a.mp3", "Un tema", "Alguien", "Un álbum"),
            1_700_000_000,
        )
        .expect("indexar");

        forget_mtime(&conn, "/m/a.mp3").expect("olvidar");

        let conocidos = known_mtimes_under(&conn, "/m").expect("consultar");
        assert_eq!(conocidos.get("/m/a.mp3"), Some(&0));
    }

    #[test]
    fn dar_de_baja_saca_el_tema_y_lo_saca_de_las_listas() {
        let (_dir, mut conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "Se va", "Alguien", "Un álbum"));
        let entrada = list_tracks(&conn).expect("list").remove(0);
        let lista = create_playlist(&conn, "Una lista").expect("create");
        add_track_to_playlist(&conn, lista.id, entrada.id).expect("add");

        let bajas = remove_tracks(&mut conn, &["/m/a.mp3".to_string()]).expect("baja");

        assert_eq!(bajas, 1);
        assert!(list_tracks(&conn).expect("list").is_empty());
        assert!(list_playlist_tracks(&conn, lista.id)
            .expect("contents")
            .is_empty());
    }

    #[test]
    fn dar_de_baja_no_borra_lo_que_se_escucho() {
        // Que un archivo se borre no borra que sonó.
        let (_dir, mut conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "Se va", "Alguien", "Un álbum"));
        record_play(&conn, "/m/a.mp3", 1_700_000_000).expect("anotar");

        remove_tracks(&mut conn, &["/m/a.mp3".to_string()]).expect("baja");

        assert_eq!(escuchas(&conn, "/m/a.mp3"), 1);
    }

    #[test]
    fn los_ajustes_de_la_biblioteca_se_guardan_y_se_releen() {
        let (_dir, conn) = temp_database();

        assert_eq!(get_setting(&conn, "scan_version"), None);

        set_setting(&conn, "scan_version", "1").expect("guardar");
        assert_eq!(get_setting(&conn, "scan_version").as_deref(), Some("1"));

        set_setting(&conn, "scan_version", "2").expect("pisar");
        assert_eq!(get_setting(&conn, "scan_version").as_deref(), Some("2"));
    }

    #[test]
    fn el_cache_de_la_ventana_no_borra_el_disco_que_leyo_el_barrido() {
        // El caché de la ventana puede ser de una versión anterior, donde estos
        // dos campos no existían, y llegar con los valores por omisión. Antes de
        // esto, abrir la aplicación pisaba con ellos lo que el barrido acababa
        // de leer del archivo.
        let (_dir, conn) = temp_database();
        indexar(
            &conn,
            &pista("/m/03.mp3", "Tercera", "Alguien", "Varios", 3),
        );

        upsert_track(&conn, &track("/m/03.mp3", "Tercera", "Alguien", "Un álbum"))
            .expect("el caché de la ventana");

        let guardado = list_tracks(&conn).expect("list").pop().expect("una fila");
        assert_eq!(guardado.track_no, 3);
        assert_eq!(guardado.album_artist, "Varios");
    }

    #[test]
    fn pero_un_disco_de_verdad_si_pisa_lo_que_habia() {
        let (_dir, conn) = temp_database();
        indexar(&conn, &track("/m/03.mp3", "Tercera", "Alguien", "Un álbum"));

        upsert_track(
            &conn,
            &pista("/m/03.mp3", "Tercera", "Alguien", "Varios", 3),
        )
        .expect("upsert");

        let guardado = list_tracks(&conn).expect("list").pop().expect("una fila");
        assert_eq!(guardado.track_no, 3);
        assert_eq!(guardado.album_artist, "Varios");
    }

    #[test]
    fn una_biblioteca_de_antes_gana_las_columnas_al_abrirse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("resonance.db");

        {
            let vieja = Connection::open(&path).expect("legacy open");
            vieja
                .execute_batch(
                    "CREATE TABLE tracks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        path TEXT NOT NULL UNIQUE,
                        title TEXT NOT NULL,
                        artist TEXT NOT NULL,
                        album TEXT NOT NULL,
                        duration_seconds INTEGER NOT NULL,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT INTO tracks (path, title, artist, album, duration_seconds)
                    VALUES ('/m/a.mp3', 'Vieja', 'Alguien', 'Un álbum', 180);",
                )
                .expect("legacy schema");
        }

        let conn = open_database(&path).expect("open");

        // Sin las columnas, cualquier consulta que las nombre falla: la prueba
        // es que esto no reviente y que los valores sean los de un archivo sin
        // esas etiquetas.
        let guardado = list_tracks(&conn).expect("list").pop().expect("una fila");
        assert_eq!(guardado.title, "Vieja");
        assert_eq!(guardado.track_no, 0);
        assert_eq!(guardado.album_artist, "");
    }

    #[test]
    fn the_same_file_is_only_indexed_once() {
        let (_dir, conn) = temp_database();
        let same = track("/m/a.mp3", "Title", "Artist", "Album");

        indexar(&conn, &same);
        indexar(&conn, &same);
        assert_eq!(list_tracks(&conn).expect("list").len(), 1);
    }

    /// Reopening must neither fail nor lose anything: a connection is opened
    /// per command, so this happens constantly.
    #[test]
    fn reopening_the_database_preserves_the_library() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("resonance.db");

        {
            let conn = open_database(&path).expect("open");
            indexar(&conn, &track("/m/a.mp3", "Title", "Artist", "Album"));
        }

        let conn = open_database(&path).expect("reopen");
        assert_eq!(list_tracks(&conn).expect("list").len(), 1);
        assert_eq!(
            search_tracks_fts(&conn, "title", 10).expect("search").len(),
            1
        );
    }

    /// A library indexed before the search index existed has tracks but nothing
    /// indexed; that is the only case where a full rebuild is warranted.
    /// Cuántas veces figura ese tema en el historial.
    fn escuchas(conn: &Connection, path: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM plays WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .expect("contar escuchas")
    }

    #[test]
    fn una_escucha_queda_anotada_con_su_momento() {
        let (_dir, conn) = temp_database();

        record_play(&conn, "/m/a.mp3", 1_700_000_000).expect("anotar");

        let (path, momento): (String, i64) = conn
            .query_row("SELECT path, played_at FROM plays", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("leer la escucha");
        assert_eq!(path, "/m/a.mp3");
        assert_eq!(momento, 1_700_000_000);
    }

    #[test]
    fn escuchar_el_mismo_tema_tres_veces_son_tres_filas() {
        // Que no se pise es todo el punto: lo que se quiere contar después es
        // cuántas veces, no si alguna vez.
        let (_dir, conn) = temp_database();

        record_play(&conn, "/m/a.mp3", 1_700_000_000).expect("anotar");
        record_play(&conn, "/m/a.mp3", 1_700_000_300).expect("anotar");
        record_play(&conn, "/m/a.mp3", 1_700_000_600).expect("anotar");

        assert_eq!(escuchas(&conn, "/m/a.mp3"), 3);
    }

    #[test]
    fn borrar_el_tema_de_la_biblioteca_no_borra_lo_que_se_escucho() {
        // Sin clave foránea a propósito: un archivo que se mueve o se borra no
        // tiene por qué llevarse puesto el registro de que sonó.
        let (_dir, conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "Un tema", "Alguien", "Un álbum"));
        record_play(&conn, "/m/a.mp3", 1_700_000_000).expect("anotar");

        conn.execute("DELETE FROM tracks WHERE path = '/m/a.mp3'", [])
            .expect("borrar el tema");

        assert_eq!(escuchas(&conn, "/m/a.mp3"), 1);
    }

    #[test]
    fn una_biblioteca_de_antes_del_historial_gana_la_tabla_al_abrirse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("resonance.db");

        {
            let vieja = Connection::open(&path).expect("legacy open");
            vieja
                .execute_batch(
                    "CREATE TABLE tracks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        path TEXT NOT NULL UNIQUE,
                        title TEXT NOT NULL,
                        artist TEXT NOT NULL,
                        album TEXT NOT NULL,
                        duration_seconds INTEGER NOT NULL,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );",
                )
                .expect("legacy schema");
        }

        let conn = open_database(&path).expect("open");

        record_play(&conn, "/m/a.mp3", 1_700_000_000).expect("anotar");
        assert_eq!(escuchas(&conn, "/m/a.mp3"), 1);
    }

    #[test]
    fn a_library_from_before_the_search_index_is_rebuilt() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("resonance.db");

        // Exactly what an old database looks like: the tracks table with rows
        // in it, and no index or triggers at all.
        {
            let legacy = Connection::open(&path).expect("legacy open");
            legacy
                .execute_batch(
                    "CREATE TABLE tracks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        path TEXT NOT NULL UNIQUE,
                        title TEXT NOT NULL,
                        artist TEXT NOT NULL,
                        album TEXT NOT NULL,
                        duration_seconds INTEGER NOT NULL,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT INTO tracks (path, title, artist, album, duration_seconds)
                    VALUES ('/m/a.mp3', 'Findable', 'Artist', 'Album', 180);",
                )
                .expect("legacy schema");
        }

        let conn = open_database(&path).expect("open");
        assert_eq!(
            search_tracks_fts(&conn, "findable", 10)
                .expect("search")
                .len(),
            1,
            "the pre-existing library should have been indexed"
        );
    }

    /// The counterpart: an up-to-date library must not pay for a rebuild. This
    /// used to run on every single connection, and a connection is opened per
    /// command — so every keystroke in the search box reindexed everything.
    #[test]
    fn an_up_to_date_library_is_not_reindexed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("resonance.db");

        {
            let conn = open_database(&path).expect("open");
            indexar(&conn, &track("/m/a.mp3", "Findable", "Artist", "Album"));
        }

        // As a fresh process would see it.
        initialised_databases().lock().unwrap().remove(&path);

        let conn = open_database(&path).expect("reopen");
        let needs_rebuild: bool = conn
            .query_row(
                "SELECT (SELECT count(*) FROM tracks) > 0
                    AND (SELECT count(*) FROM tracks_fts_docsize) = 0",
                [],
                |row| row.get(0),
            )
            .expect("check");

        assert!(
            !needs_rebuild,
            "the triggers already keep the index current"
        );
        assert_eq!(
            search_tracks_fts(&conn, "findable", 10)
                .expect("search")
                .len(),
            1
        );
    }

    #[test]
    fn playlists_hold_their_tracks_in_order() {
        let (_dir, conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "First", "Artist", "Album"));
        indexar(&conn, &track("/m/b.mp3", "Second", "Artist", "Album"));

        let tracks = list_tracks(&conn).expect("list");
        let playlist = create_playlist(&conn, "Road trip").expect("create");

        for entry in &tracks {
            add_track_to_playlist(&conn, playlist.id, entry.id).expect("add");
        }

        let contents = list_playlist_tracks(&conn, playlist.id).expect("contents");
        assert_eq!(contents.len(), 2);

        remove_track_from_playlist(&conn, playlist.id, tracks[0].id).expect("remove");
        assert_eq!(
            list_playlist_tracks(&conn, playlist.id)
                .expect("contents")
                .len(),
            1
        );

        // Deleting the playlist must not take the tracks with it.
        delete_playlist(&conn, playlist.id).expect("delete");
        assert!(list_playlists(&conn).expect("list").is_empty());
        assert_eq!(list_tracks(&conn).expect("tracks").len(), 2);
    }

    /// Deleting a track has to clear it from the index and from any playlist,
    /// or searches keep returning a row that no longer exists.
    #[test]
    fn deleting_a_track_removes_it_everywhere() {
        let (_dir, conn) = temp_database();
        indexar(&conn, &track("/m/a.mp3", "Doomed", "Artist", "Album"));
        let entry = list_tracks(&conn).expect("list").remove(0);

        let playlist = create_playlist(&conn, "List").expect("create");
        add_track_to_playlist(&conn, playlist.id, entry.id).expect("add");

        conn.execute("DELETE FROM tracks WHERE id = ?1", params![entry.id])
            .expect("delete");

        assert!(search_tracks_fts(&conn, "doomed", 10)
            .expect("search")
            .is_empty());
        assert!(list_playlist_tracks(&conn, playlist.id)
            .expect("contents")
            .is_empty());
    }
}
