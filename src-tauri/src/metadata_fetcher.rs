use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine;
const DEEZER_BASE_URL: &str = "https://api.deezer.com";
const MUSICBRAINZ_BASE_URL: &str = "https://musicbrainz.org/ws/2";
const COVERARTARCHIVE_BASE_URL: &str = "https://coverartarchive.org";

const ALBUM_CACHE_RELATIVE_PATH: &str = ".cache/resonance/albums";

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

fn album_cache_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)?;
    Some(home.join(ALBUM_CACHE_RELATIVE_PATH))
}

/// Normalize album name: lowercase, alphanumeric + hyphens only
fn normalize_album_name(album: &str) -> String {
    album
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .trim_matches('-')
        .to_string()
}

/// Get cached album cover path if it exists
fn get_cached_cover_path(artist: &str, album: &str) -> Option<PathBuf> {
    let cache_dir = album_cache_dir()?;
    let normalized = format!(
        "{}-{}",
        normalize_album_name(artist),
        normalize_album_name(album)
    );

    // Check for .jpg or .png
    let jpg_path = cache_dir.join(format!("{}.jpg", normalized));
    if jpg_path.exists() {
        return Some(jpg_path);
    }

    let png_path = cache_dir.join(format!("{}.png", normalized));
    if png_path.exists() {
        return Some(png_path);
    }

    None
}

#[derive(Debug, Deserialize)]
struct DeezerAlbumSearchResult {
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    title: String,
    cover_big: Option<String>,
    cover_xl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeezerSearchResponse {
    data: Vec<DeezerAlbumSearchResult>,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzRelease {
    id: String,
    #[allow(dead_code)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct MusicBrainzSearchResponse {
    releases: Vec<MusicBrainzRelease>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct CoverArtArchiveResponse {
    images: Vec<CoverImage>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct CoverImage {
    front: bool,
    back: bool,
    image: String,
}

/// Try to fetch album cover from Deezer API
async fn fetch_from_deezer(artist: &str, album: &str) -> Option<String> {
    let query = format!("{} {}", artist, album);
    let url = format!(
        "{}/search/album?q={}",
        DEEZER_BASE_URL,
        urlencoding::encode(&query)
    );

    let response = http_client()
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;

    let data: DeezerSearchResponse = response.json().await.ok()?;

    // Get first result with cover image
    data.data
        .iter()
        .find(|album_result| album_result.cover_xl.is_some() || album_result.cover_big.is_some())
        .and_then(|album_result| {
            album_result
                .cover_xl
                .as_ref()
                .or(album_result.cover_big.as_ref())
                .cloned()
        })
}

/// Try to fetch album cover from MusicBrainz API (via CoverArtArchive)
async fn fetch_from_musicbrainz(artist: &str, album: &str) -> Option<String> {
    // Step 1: Search for release on MusicBrainz
    let query = format!("artist:{} AND release:{}", artist, album);
    let url = format!(
        "{}/release?query={}&fmt=json",
        MUSICBRAINZ_BASE_URL,
        urlencoding::encode(&query)
    );

    let response = http_client()
        .get(&url)
        .header("User-Agent", "vasak-resonance/1.0")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;

    let data: MusicBrainzSearchResponse = response.json().await.ok()?;

    let release_id = data.releases.first()?.id.clone();

    // Step 2: Get cover art from CoverArtArchive
    let cover_url = format!("{}/release/{}/front", COVERARTARCHIVE_BASE_URL, release_id);

    // Check if cover exists with HEAD request
    let head_response = http_client()
        .head(&cover_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;

    if head_response.status().is_success() {
        return Some(cover_url);
    }

    None
}

/// Download image from URL and return bytes
async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let response = http_client()
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read image bytes: {}", e))
}

/// Determine image format from response headers or URL
fn guess_image_format(content_type: Option<&str>, url: &str) -> &'static str {
    if let Some(ct) = content_type {
        if ct.contains("png") {
            return "png";
        }
        if ct.contains("jpeg") || ct.contains("jpg") {
            return "jpg";
        }
    }

    // Fallback: guess from URL
    if url.contains(".png") {
        "png"
    } else {
        "jpg"
    }
}

/// Convert image file to data URL (base64 encoded)
/// La portada y el color que la representa.
///
/// Van juntos porque los bytes de la imagen ya están de este lado: el frontend
/// los volvía a decodificar en un `<canvas>` sólo para promediar píxeles.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PortadaConColor {
    pub cover_data_url: String,
    /// `#RRGGBB`, o vacío si la imagen no se pudo leer.
    pub dominant_color: String,
    /// De dónde se bajó la imagen, o vacío si no se sabe.
    ///
    /// La aplicación no la necesita —tiene los bytes—, pero Discord sí: dibuja
    /// la tarjeta desde su lado y sólo llega a direcciones web, así que una
    /// tapa incrustada en el archivo o cacheada en el disco no le sirve. Es el
    /// único dato de todo este módulo que sale del equipo, y por eso se guarda
    /// y se relee pasando por [`host_permitido`].
    pub remote_url: String,
}

/// Los únicos servidores de los que puede venir una dirección que se comparta.
///
/// Son los dos de los que este módulo baja portadas. La comprobación no es por
/// desconfiar de las APIs sino del archivo `.url` que queda en el disco: es un
/// archivo de texto en la carpeta de caché de quien usa el equipo, y lo que
/// diga termina en un servicio ajeno.
fn host_permitido(url: &str) -> bool {
    let Some(resto) = url.strip_prefix("https://") else {
        return false;
    };

    let host = resto
        .split('/')
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    host == "coverartarchive.org" || host.ends_with(".dzcdn.net")
}

/// El archivo de al lado donde queda anotada la dirección de la portada.
///
/// Un archivo suelto y no una base: la caché de portadas ya son archivos con el
/// nombre normalizado del álbum, y borrar la carpeta tiene que seguir dejando
/// todo consistente.
fn cover_url_sidecar_path(artist: &str, album: &str) -> Option<PathBuf> {
    let cache_dir = album_cache_dir()?;
    let normalized = format!(
        "{}-{}",
        normalize_album_name(artist),
        normalize_album_name(album)
    );
    Some(cache_dir.join(format!("{}.url", normalized)))
}

/// Lee el archivo de al lado. Vacío si no hay, no se puede leer, o dice algo
/// que no se puede compartir.
fn leer_url_del_archivo(path: &Path) -> String {
    let Ok(contenido) = fs::read_to_string(path) else {
        return String::new();
    };

    let url = contenido.trim();
    if host_permitido(url) {
        url.to_string()
    } else {
        String::new()
    }
}

/// La dirección anotada para ese álbum, si hay y si todavía es aceptable.
fn leer_url_guardada(artist: &str, album: &str) -> String {
    cover_url_sidecar_path(artist, album)
        .map(|path| leer_url_del_archivo(&path))
        .unwrap_or_default()
}

/// Anota la dirección al lado de la imagen. Que falle no invalida la portada.
fn guardar_url(artist: &str, album: &str, url: &str) {
    if !host_permitido(url) {
        return;
    }

    if let Some(path) = cover_url_sidecar_path(artist, album) {
        let _ = fs::write(path, url);
    }
}

fn file_to_data_url(
    path: &PathBuf,
    format: &str,
    remote_url: String,
) -> Result<PortadaConColor, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read image file: {}", e))?;

    // El color se calcula acá, con la misma paleta que las portadas embebidas.
    // Que falle no invalida la portada: se devuelve sin color y la vista usa el
    // suyo por omisión.
    let dominant_color = crate::audio::extract_dominant_color_hex(&bytes).unwrap_or_default();

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mime_type = match format {
        "png" => "image/png",
        _ => "image/jpeg",
    };

    Ok(PortadaConColor {
        cover_data_url: format!("data:{};base64,{}", mime_type, encoded),
        dominant_color,
        remote_url,
    })
}
/// Main function: Fetch and cache album cover
pub async fn fetch_album_cover(artist: String, album: String) -> Result<PortadaConColor, String> {
    // Check cache first
    if let Some(cached_path) = get_cached_cover_path(&artist, &album) {
        // Convert cached file to data URL
        let format = if cached_path.extension().and_then(|ext| ext.to_str()) == Some("png") {
            "png"
        } else {
            "jpg"
        };
        // Una caché de antes de que esto existiera no tiene el archivo de al
        // lado: la portada se sirve igual y Discord cae en el logo del sistema,
        // que es lo que hacía siempre.
        return file_to_data_url(&cached_path, format, leer_url_guardada(&artist, &album));
    }

    // Try Deezer first (faster, single request)
    let image_url = if let Some(url) = fetch_from_deezer(&artist, &album).await {
        url
    } else if let Some(url) = fetch_from_musicbrainz(&artist, &album).await {
        // MusicBrainz as fallback
        url
    } else {
        return Err("Could not find album cover on Deezer or MusicBrainz".to_string());
    };

    // Download image
    let image_bytes = download_image(&image_url).await?;

    // Determine format
    let format = guess_image_format(None, &image_url);

    // Ensure cache directory exists
    let cache_dir =
        album_cache_dir().ok_or_else(|| "Could not determine cache directory".to_string())?;

    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Generate cache filename
    let normalized = format!(
        "{}-{}",
        normalize_album_name(&artist),
        normalize_album_name(&album)
    );
    let cache_file = cache_dir.join(format!("{}.{}", normalized, format));

    // Write to cache
    fs::write(&cache_file, image_bytes)
        .map_err(|e| format!("Failed to write image to cache: {}", e))?;

    // Queda anotada para la próxima, que va a salir de la caché y ya no va a
    // tener de dónde sacarla.
    guardar_url(&artist, &album, &image_url);

    // Return as data URL
    let remote_url = if host_permitido(&image_url) {
        image_url
    } else {
        String::new()
    };

    file_to_data_url(&cache_file, format, remote_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La dirección de la portada es lo único de este módulo que sale del
    /// equipo: se la lleva Discord, que la dibuja desde su lado. Lo que entra en
    /// el archivo de al lado tiene que sobrevivir a que alguien lo edite.
    #[test]
    fn solo_pasan_los_dos_servidores_de_los_que_se_bajan_portadas() {
        assert!(host_permitido(
            "https://e-cdns-images.dzcdn.net/images/cover/abc/1000x1000-000000-80-0-0.jpg"
        ));
        assert!(host_permitido(
            "https://cdn-images.dzcdn.net/images/cover/abc/500x500.jpg"
        ));
        assert!(host_permitido(
            "https://coverartarchive.org/release/abc/front"
        ));
    }

    #[test]
    fn no_pasa_nada_mas() {
        for url in [
            // Sin cifrar: la dirección viaja y la imagen también.
            "http://coverartarchive.org/release/abc/front",
            // Un archivo del disco no es una dirección que Discord pueda ver, y
            // además diría dónde vive la música de quien usa el equipo.
            "file:///home/alguien/Música/tapa.jpg",
            "data:image/png;base64,AAAA",
            // Otro servidor cualquiera.
            "https://ejemplo.invalido/tapa.jpg",
            // El nombre permitido, pero de adentro de otro dominio.
            "https://coverartarchive.org.ejemplo.invalido/tapa.jpg",
            "https://dzcdn.net.ejemplo.invalido/tapa.jpg",
            // El truco del arroba: el host de verdad es el de la derecha.
            "https://coverartarchive.org@ejemplo.invalido/tapa.jpg",
            "",
        ] {
            assert!(!host_permitido(url), "{url}");
        }
    }

    #[test]
    fn la_direccion_sobrevive_a_que_la_portada_salga_de_la_cache() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let sidecar = dir.path().join("queen-a-night-at-the-opera.url");
        let url = "https://coverartarchive.org/release/abc/front";

        fs::write(&sidecar, url).expect("no se pudo escribir");

        assert_eq!(leer_url_del_archivo(&sidecar), url);
    }

    #[test]
    fn una_cache_vieja_no_tiene_direccion_y_no_es_un_error() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");

        assert_eq!(leer_url_del_archivo(&dir.path().join("no-existe.url")), "");
    }

    #[test]
    fn un_archivo_manoseado_no_llega_a_discord() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let sidecar = dir.path().join("manoseado.url");

        fs::write(&sidecar, "file:///etc/passwd").expect("no se pudo escribir");

        assert_eq!(leer_url_del_archivo(&sidecar), "");
    }

    #[test]
    fn los_espacios_de_mas_no_invalidan_la_direccion() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let sidecar = dir.path().join("con-salto.url");
        let url = "https://cdn-images.dzcdn.net/images/cover/abc/500x500.jpg";

        fs::write(&sidecar, format!("{url}\n")).expect("no se pudo escribir");

        assert_eq!(leer_url_del_archivo(&sidecar), url);
    }
}
