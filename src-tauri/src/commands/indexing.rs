use std::path::PathBuf;
use walkdir::WalkDir;

use crate::audio::{extract_track_from_file, is_supported_audio_file};
use crate::db::{get_database_path, insert_track_if_not_exists, open_database};
use crate::structs::ScanSummary;

#[tauri::command]
pub fn scan_music_folders(folders: Vec<String>) -> Result<ScanSummary, String> {
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
