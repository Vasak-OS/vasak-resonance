use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: Option<i64>,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// El artista **del álbum**, que puede no ser el del tema.
    ///
    /// Es la etiqueta que existe para las recopilaciones: cada pista tiene su
    /// intérprete y todas comparten el disco. Sin ella, agrupar por artista y
    /// álbum parte una recopilación en un álbum por intérprete.
    ///
    /// Vacío cuando el archivo no la trae, que es lo normal en un disco de un
    /// solo artista; ahí el del tema alcanza.
    pub album_artist: String,
    /// El número de pista dentro del álbum, o 0 si el archivo no lo dice.
    pub track_no: i64,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrack {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Ver [`Track::album_artist`].
    pub album_artist: String,
    /// Ver [`Track::track_no`].
    pub track_no: i64,
    pub duration_seconds: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub scanned_files: usize,
    /// Temas que no estaban en la biblioteca.
    pub inserted_tracks: usize,
    /// Temas que ya estaban y cuyo archivo cambió desde la última vez.
    pub updated_tracks: usize,
    /// Temas que ya estaban y cuyo archivo no cambió: no se abrieron siquiera.
    pub unchanged_tracks: usize,
    /// Temas que estaban en la biblioteca y ya no en el disco.
    pub removed_tracks: usize,
    pub skipped_non_audio: usize,
    pub failed_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedPlaybackTrack {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Ver [`Track::album_artist`].
    pub album_artist: String,
    /// Ver [`Track::track_no`].
    pub track_no: i64,
    pub duration_seconds: i64,
    pub cover_data_url: Option<String>,
    pub dominant_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingMetadata {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Ver [`Track::album_artist`].
    pub album_artist: String,
    /// Ver [`Track::track_no`].
    pub track_no: i64,
    pub duration_seconds: u64,
    pub cover_data_url: Option<String>,
    pub dominant_color: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackProgressEvent {
    pub path: Option<String>,
    pub position_seconds: u64,
    pub duration_seconds: Option<u64>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub volume: f32,
    /// Behind an `Arc` because this snapshot is cloned twice per tick — once
    /// into the shared state and once into the emitted event — and
    /// `cover_data_url` is a base64 image, routinely hundreds of kilobytes. As
    /// a plain field that was a megabyte or so of memcpy and allocator churn
    /// every second, for a value that only changes when the track does.
    /// Serialisation is unaffected: serde sees straight through an `Arc`.
    pub now_playing: Option<Arc<NowPlayingMetadata>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLine {
    pub time_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackLyricsPayload {
    pub source: String,
    pub synced: bool,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub lines: Vec<LyricsLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTrack {
    pub playlist_id: i64,
    pub track_id: i64,
    pub position: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: i64,
}
