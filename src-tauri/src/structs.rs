use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: Option<i64>,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrack {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_seconds: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub scanned_files: usize,
    pub inserted_tracks: usize,
    pub skipped_duplicates: usize,
    pub skipped_non_audio: usize,
    pub failed_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedPlaybackTrack {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
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
    pub now_playing: Option<NowPlayingMetadata>,
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
