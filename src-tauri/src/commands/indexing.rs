use std::collections::HashSet;
use std::path::PathBuf;
use tauri::Emitter;
use walkdir::WalkDir;

use crate::audio::{extract_track_from_file, is_supported_audio_file};
use crate::db::{get_database_path, insert_track_if_not_exists, open_database};
use crate::structs::ScanSummary;

fn scan_folders_internal(folders: &[String]) -> Result<ScanSummary, String> {
    let db_path = get_database_path()?;
    let conn = open_database(&db_path)?;

    let mut summary = ScanSummary {
        scanned_files: 0,
        inserted_tracks: 0,
        skipped_duplicates: 0,
        skipped_non_audio: 0,
        failed_files: 0,
    };

    for folder in folders {
        let root = PathBuf::from(folder);
        if !root.exists() || !root.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            summary.scanned_files += 1;

            if !is_supported_audio_file(path) {
                summary.skipped_non_audio += 1;
                continue;
            }

            match extract_track_from_file(path) {
                Ok(track) => {
                    if insert_track_if_not_exists(&conn, &track)? {
                        summary.inserted_tracks += 1;
                    } else {
                        summary.skipped_duplicates += 1;
                    }
                }
                Err(_) => {
                    summary.failed_files += 1;
                }
            }
        }
    }

    Ok(summary)
}

fn resolve_default_music_folders() -> Vec<String> {
    let mut ordered = Vec::<PathBuf>::new();
    let mut seen = HashSet::<PathBuf>::new();

    let mut push_if_valid = |candidate: PathBuf| {
        if candidate.exists() && candidate.is_dir() {
            let canonical = std::fs::canonicalize(&candidate).unwrap_or(candidate.clone());
            if seen.insert(canonical.clone()) {
                ordered.push(canonical);
            }
        }
    };

    if let Some(audio_dir) = dirs::audio_dir() {
        push_if_valid(audio_dir);
    }

    if let Some(home) = dirs::home_dir() {
        // Fallbacks para sistemas donde xdg-user-dirs no esté configurado.
        let common_candidates = [
            "Music",
            "Música",
            "Musica",
            "musica",
            "music",
            "MUSICA",
            "MUSICA",
            "Musik",
            "musik",
            "Muzyka",
            "Muzica",
            "Musique",
            "Musik",
            "Музыка",
        ];

        for name in common_candidates {
            push_if_valid(home.join(name));
        }
    }

    ordered
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

#[tauri::command]
pub fn scan_music_folders(folders: Vec<String>) -> Result<ScanSummary, String> {
    scan_folders_internal(&folders)
}

#[tauri::command]
pub async fn scan_default_music_folder(
    app_handle: tauri::AppHandle,
) -> Result<ScanSummary, String> {
    let folders = resolve_default_music_folders();
    let result = scan_folders_internal(&folders)?;

    // Emit scan complete event
    let _ = app_handle.emit("scan-complete", &result);

    Ok(result)
}
