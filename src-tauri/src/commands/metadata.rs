use crate::metadata_fetcher::fetch_album_cover;

#[tauri::command]
pub async fn fetch_album_cover_command(artist: String, album: String) -> Result<String, String> {
    fetch_album_cover(artist, album).await
}
