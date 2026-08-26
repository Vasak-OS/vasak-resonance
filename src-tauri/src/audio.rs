use base64::{engine::general_purpose, Engine as _};
use color_thief::{get_palette, ColorFormat};
use image::ImageReader;
use lofty::picture::PictureType;
use lofty::prelude::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::structs::{NowPlayingMetadata, Track};

pub fn extract_track_from_file(path: &Path) -> Result<Track, String> {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let duration_seconds = tagged_file.properties().duration().as_secs() as i64;

    Ok(Track {
        id: None,
        path: canonical_path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_seconds,
    })
}

pub fn extract_now_playing_metadata(path: &Path) -> Result<NowPlayingMetadata, String> {
    let mut cover_cache = HashMap::<String, Option<String>>::new();
    let mut dominant_color_cache = HashMap::<String, Option<String>>::new();
    extract_now_playing_metadata_with_cover_cache(path, &mut cover_cache, &mut dominant_color_cache)
}

pub fn extract_now_playing_metadata_with_cover_cache(
    path: &Path,
    cover_cache: &mut HashMap<String, Option<String>>,
    dominant_color_cache: &mut HashMap<String, Option<String>>,
) -> Result<NowPlayingMetadata, String> {
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let canonical_path_str = canonical_path.to_string_lossy().to_string();

    let tagged_file = Probe::open(path)
        .map_err(|e| format!("No se pudo abrir archivo de audio {}: {e}", path.display()))?
        .read()
        .map_err(|e| format!("No se pudo leer metadata de {}: {e}", path.display()))?;

    let primary_tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let fallback_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    let duration_seconds = tagged_file.properties().duration().as_secs();

    let mut computed_cover_data_url: Option<String> = None;
    let mut computed_dominant_color: Option<String> = None;

    let (cover_data_url, dominant_color) = if let Some(cached_cover) =
        cover_cache.get(&canonical_path_str)
    {
        (
            cached_cover.clone(),
            dominant_color_cache
                .get(&canonical_path_str)
                .cloned()
                .unwrap_or(None),
        )
    } else {
        if let Some(tag) = primary_tag {
            if let Some(picture) = tag
                .get_picture_type(PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
            {
                let mime = picture
                    .mime_type()
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "image/jpeg".to_string());
                let encoded = general_purpose::STANDARD.encode(picture.data());
                computed_cover_data_url = Some(format!("data:{};base64,{}", mime, encoded));
                computed_dominant_color = extract_dominant_color_hex(picture.data());
            }
        }

        cover_cache.insert(canonical_path_str.clone(), computed_cover_data_url.clone());
        dominant_color_cache.insert(canonical_path_str.clone(), computed_dominant_color.clone());

        (computed_cover_data_url, computed_dominant_color)
    };

    Ok(NowPlayingMetadata {
        path: canonical_path_str,
        title,
        artist,
        album,
        duration_seconds,
        cover_data_url,
        dominant_color,
    })
}

/// Color dominante de una imagen, en `#RRGGBB`.
///
/// Público porque también lo necesita la portada bajada de la red: el frontend
/// la dibujaba en un `<canvas>` de 48x48 y promediaba los píxeles a mano, o sea
/// una decodificación de imagen y un recorrido de 9216 píxeles en el hilo que
/// dibuja, y con un algoritmo peor —un promedio, no una paleta— que el que ya
/// estaba acá para las portadas embebidas.
pub fn extract_dominant_color_hex(image_data: &[u8]) -> Option<String> {
    let cursor = Cursor::new(image_data);
    let decoded = ImageReader::new(cursor)
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let rgba = decoded.to_rgba8();

    let palette = get_palette(rgba.as_raw(), ColorFormat::Rgba, 10, 5).ok()?;
    let color = palette.first()?;

    Some(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b))
}

pub fn is_supported_audio_file(path: &Path) -> bool {
    const SUPPORTED_EXTENSIONS: &[&str] = &[
        "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "opus", "wma", "aiff", "alac",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un PNG de un solo color, armado en memoria.
    fn png_de_un_color(r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut imagen = image::RgbImage::new(24, 24);
        for pixel in imagen.pixels_mut() {
            *pixel = image::Rgb([r, g, b]);
        }
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(imagen)
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("el PNG de prueba tiene que poder escribirse");
        bytes
    }

    #[test]
    fn el_color_dominante_sale_en_hexadecimal_de_seis_digitos() {
        // El formato importa: la interfaz lo mete tal cual en una propiedad CSS,
        // así que un `#RGB` o un `rgb()` no servirían.
        let color = extract_dominant_color_hex(&png_de_un_color(0xFF, 0x00, 0x00))
            .expect("una imagen válida tiene que dar un color");
        assert!(color.starts_with('#'), "{color}");
        assert_eq!(color.len(), 7, "{color}");
        assert!(
            color[1..].chars().all(|c| c.is_ascii_hexdigit()),
            "{color}"
        );
    }

    #[test]
    fn una_imagen_roja_da_un_color_rojizo() {
        let color = extract_dominant_color_hex(&png_de_un_color(0xE0, 0x10, 0x10)).unwrap();
        let rojo = u8::from_str_radix(&color[1..3], 16).unwrap();
        let verde = u8::from_str_radix(&color[3..5], 16).unwrap();
        let azul = u8::from_str_radix(&color[5..7], 16).unwrap();

        assert!(rojo > verde && rojo > azul, "{color} no es rojizo");
    }

    #[test]
    fn una_imagen_azul_da_un_color_azulado() {
        // Con el promedio a mano que hacía el frontend esto también pasaba; lo
        // que se gana es que el algoritmo sea uno solo y el mejor de los dos.
        let color = extract_dominant_color_hex(&png_de_un_color(0x10, 0x20, 0xE0)).unwrap();
        let rojo = u8::from_str_radix(&color[1..3], 16).unwrap();
        let azul = u8::from_str_radix(&color[5..7], 16).unwrap();

        assert!(azul > rojo, "{color} no es azulado");
    }

    #[test]
    fn los_bytes_que_no_son_una_imagen_no_dan_color() {
        // Una descarga cortada o un HTML de error en lugar de la imagen: tiene
        // que devolver None y no reventar, porque de esto depende que la portada
        // se muestre igual sin color.
        assert!(extract_dominant_color_hex(b"no soy una imagen").is_none());
        assert!(extract_dominant_color_hex(&[]).is_none());
        assert!(extract_dominant_color_hex(b"<html>404</html>").is_none());
    }

    #[test]
    fn un_png_truncado_no_paniquea() {
        // Media descarga es el caso realista: el decodificador tiene que fallar
        // devolviendo None, no abortando el comando.
        let completo = png_de_un_color(0x40, 0x80, 0xC0);
        let mitad = &completo[..completo.len() / 2];
        assert!(extract_dominant_color_hex(mitad).is_none());
    }

    #[test]
    fn se_reconocen_los_formatos_de_audio_soportados() {
        // La lista decide qué entra a la biblioteca al escanear; una extensión
        // en mayúsculas es lo que más se ve en archivos viejos.
        assert!(is_supported_audio_file(Path::new("/m/tema.mp3")));
        assert!(is_supported_audio_file(Path::new("/m/tema.FLAC")));
        assert!(is_supported_audio_file(Path::new("/m/tema.Opus")));
        assert!(!is_supported_audio_file(Path::new("/m/tapa.jpg")));
        assert!(!is_supported_audio_file(Path::new("/m/sin-extension")));
    }
}
