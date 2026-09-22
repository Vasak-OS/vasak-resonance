use base64::{engine::general_purpose, Engine as _};
use color_thief::{get_palette, ColorFormat};
use image::ImageReader;
use lofty::picture::PictureType;
use lofty::prelude::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::structs::{NowPlayingMetadata, Track};

/// Los nombres con los que se guarda la tapa al lado de la música, en orden de
/// preferencia.
///
/// Se comparan **sin mirar mayúsculas**: `Folder.jpg` con mayúscula es lo que
/// deja medio Windows, y en Linux ése es otro archivo.
const NOMBRES_DE_TAPA: [&str; 6] = [
    "cover.jpg",
    "cover.png",
    "folder.jpg",
    "folder.png",
    "front.jpg",
    "album.jpg",
];

/// Techo de lo que se lee como tapa.
///
/// Una tapa son unos cientos de kilobytes. El techo no es por las tapas sino
/// por lo que puede haber quedado con ese nombre: el escaneo de la contratapa a
/// 600 puntos por pulgada no vale la pena para pintar un cuadradito, y encima
/// viaja a la ventana en base64.
const MAXIMO_BYTES_DE_TAPA: u64 = 16 * 1024 * 1024;

/// Cuántas entradas de una carpeta se miran buscando la tapa.
///
/// Una carpeta de álbum tiene decenas. Mil es de sobra para cualquier disco, y
/// evita recorrer entera una carpeta de descargas con cien mil archivos donde la
/// tapa no va a estar igual.
const MAXIMO_ENTRADAS_MIRADAS: usize = 1000;

/// La tapa que está al lado de los archivos, ya leída.
#[derive(Debug, Clone)]
pub struct TapaDeCarpeta {
    pub data_url: String,
    pub dominant_color: Option<String>,
}

/// Lo leído por carpeta, para no volver a buscar ni a decodificar la misma
/// imagen una vez por cada tema del disco.
pub type TapaPorCarpeta = HashMap<String, Option<TapaDeCarpeta>>;

/// El archivo de tapa de una carpeta, si hay alguno.
///
/// Una sola pasada por la carpeta en vez de un `stat` por cada nombre posible:
/// así se reconoce cualquier combinación de mayúsculas sin multiplicar las
/// consultas, que es donde está la mitad de las tapas del mundo real.
fn archivo_de_tapa(carpeta: &Path) -> Option<PathBuf> {
    let entradas = std::fs::read_dir(carpeta).ok()?;
    let mut mejor: Option<(usize, PathBuf)> = None;

    for entrada in entradas.take(MAXIMO_ENTRADAS_MIRADAS).flatten() {
        let nombre = entrada.file_name().to_string_lossy().to_ascii_lowercase();
        let Some(prioridad) = NOMBRES_DE_TAPA
            .iter()
            .position(|candidato| *candidato == nombre)
        else {
            continue;
        };

        if mejor
            .as_ref()
            .is_some_and(|(anterior, _)| *anterior <= prioridad)
        {
            continue;
        }

        mejor = Some((prioridad, entrada.path()));

        // La primera de la lista: no hay nada mejor que buscar.
        if prioridad == 0 {
            break;
        }
    }

    mejor.map(|(_, path)| path)
}

/// La tapa que está al lado del archivo de audio, leída y con su color.
///
/// Es la única fuente de tapas que funciona sin conexión y sin equivocarse: es
/// la imagen que puso quien armó la carpeta. Se mira cuando el archivo no trae
/// ninguna incrustada.
fn tapa_de_la_carpeta(audio: &Path, cache: &mut TapaPorCarpeta) -> Option<TapaDeCarpeta> {
    let carpeta = audio.parent()?;
    let clave = carpeta.to_string_lossy().to_string();

    if let Some(cacheada) = cache.get(&clave) {
        return cacheada.clone();
    }

    let leida = archivo_de_tapa(carpeta).and_then(|imagen| {
        let metadata = std::fs::metadata(&imagen).ok()?;
        if !metadata.is_file() || metadata.len() > MAXIMO_BYTES_DE_TAPA {
            return None;
        }

        let bytes = fs::read(&imagen).ok()?;
        if bytes.is_empty() {
            return None;
        }

        let mime = if imagen
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            "image/png"
        } else {
            "image/jpeg"
        };

        Some(TapaDeCarpeta {
            data_url: format!(
                "data:{};base64,{}",
                mime,
                general_purpose::STANDARD.encode(&bytes)
            ),
            dominant_color: extract_dominant_color_hex(&bytes),
        })
    });

    cache.insert(clave, leida.clone());
    leida
}

/// Lo que se guarda cuando la etiqueta no lo dice.
///
/// Son constantes y no literales sueltos porque hay quien tiene que
/// **reconocerlos**: el scrobbling no manda a Last.fm un tema cuyo artista es
/// esto, y un perfil lleno de «Unknown Artist» es peor que un hueco. Dos copias
/// a mano de la misma cadena son dos copias que se separan.
pub const TITULO_DESCONOCIDO: &str = "Unknown Title";
pub const ARTISTA_DESCONOCIDO: &str = "Unknown Artist";
pub const ALBUM_DESCONOCIDO: &str = "Unknown Album";

/// Con qué reglas se leyeron las etiquetas de la biblioteca.
///
/// **Se sube cuando esta función empieza a producir algo distinto** de lo que
/// producía antes: un título, un artista o un campo nuevo. El barrido compara
/// este número con el que quedó anotado en la base y, si no coinciden, relee
/// todos los archivos una vez en vez de confiar en la fecha de modificación.
///
/// Sin esto, cada cambio de estas reglas necesita su propio apaño para que la
/// biblioteca ya indexada se entere — que es lo que hubo que hacer al empezar a
/// leer el número de pista y el artista del álbum.
pub const SCAN_VERSION: &str = "1";

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
        .unwrap_or(TITULO_DESCONOCIDO)
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| ARTISTA_DESCONOCIDO.to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| ALBUM_DESCONOCIDO.to_string());

    // El artista del álbum: lo que mantiene junta una recopilación. `lofty` lo
    // expone como texto libre; vacío es «no lo dice», y entonces el del tema
    // alcanza.
    let album_artist = primary_tag
        .and_then(|tag| tag.get_string(&ItemKey::AlbumArtist))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();

    // El número de pista. Cero es «sin numerar», y es lo que hace que esos
    // archivos queden al final en vez de mezclados.
    let track_no = primary_tag.and_then(|tag| tag.track()).unwrap_or(0) as i64;

    let duration_seconds = tagged_file.properties().duration().as_secs() as i64;

    Ok(Track {
        id: None,
        path: canonical_path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        album_artist,
        track_no,
        duration_seconds,
    })
}

/// Las tapas de carpeta que ya se leyeron, compartidas por todo el proceso.
///
/// La ventana pide los datos de cada tema por separado —al recargar la
/// biblioteca, uno por uno—, así que un caché local a la llamada no sirve de
/// nada: un disco de veinte temas leería y decodificaría veinte veces la misma
/// imagen. Acá se comparten.
static TAPAS_DE_CARPETA: OnceLock<Mutex<TapaPorCarpeta>> = OnceLock::new();

/// Cuántas carpetas se recuerdan.
///
/// Cada entrada puede llevar una imagen entera en base64, así que se vacía
/// entero al pasarse, igual que el caché del hilo de audio: perderlo cuesta una
/// relectura, no un error.
const MAXIMO_CARPETAS_RECORDADAS: usize = 64;

pub fn extract_now_playing_metadata(path: &Path) -> Result<NowPlayingMetadata, String> {
    let mut cover_cache = HashMap::<String, Option<String>>::new();
    let mut dominant_color_cache = HashMap::<String, Option<String>>::new();

    // El candado se toma dos veces y corto, en vez de una sola vez alrededor de
    // toda la extracción: leer y parsear las etiquetas es lo caro, y la ventana
    // pide veinte temas a la vez. Con el candado abierto todo ese rato, esos
    // veinte pedidos se harían de a uno.
    let carpeta = path
        .parent()
        .map(|carpeta| carpeta.to_string_lossy().to_string());
    let mut tapa_por_carpeta = TapaPorCarpeta::new();
    if let Some(carpeta) = &carpeta {
        if let Some(cacheada) = tapas_de_carpeta()
            .lock()
            .ok()
            .and_then(|cache| cache.get(carpeta).cloned())
        {
            tapa_por_carpeta.insert(carpeta.clone(), cacheada);
        }
    }

    let metadata = extract_now_playing_metadata_with_cover_cache(
        path,
        &mut cover_cache,
        &mut dominant_color_cache,
        &mut tapa_por_carpeta,
    );

    if let (Some(carpeta), Ok(mut compartido)) = (carpeta, tapas_de_carpeta().lock()) {
        if let Some(leida) = tapa_por_carpeta.remove(&carpeta) {
            if compartido.len() >= MAXIMO_CARPETAS_RECORDADAS {
                compartido.clear();
            }
            compartido.insert(carpeta, leida);
        }
    }

    metadata
}

fn tapas_de_carpeta() -> &'static Mutex<TapaPorCarpeta> {
    TAPAS_DE_CARPETA.get_or_init(|| Mutex::new(TapaPorCarpeta::new()))
}

pub fn extract_now_playing_metadata_with_cover_cache(
    path: &Path,
    cover_cache: &mut HashMap<String, Option<String>>,
    dominant_color_cache: &mut HashMap<String, Option<String>>,
    tapa_por_carpeta: &mut TapaPorCarpeta,
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
        .unwrap_or(TITULO_DESCONOCIDO)
        .to_string();

    let title = primary_tag
        .and_then(|tag| tag.title().map(|v| v.to_string()))
        .unwrap_or(fallback_name);

    let artist = primary_tag
        .and_then(|tag| tag.artist().map(|v| v.to_string()))
        .unwrap_or_else(|| ARTISTA_DESCONOCIDO.to_string());

    let album = primary_tag
        .and_then(|tag| tag.album().map(|v| v.to_string()))
        .unwrap_or_else(|| ALBUM_DESCONOCIDO.to_string());

    let album_artist = primary_tag
        .and_then(|tag| tag.get_string(&ItemKey::AlbumArtist))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let track_no = primary_tag.and_then(|tag| tag.track()).unwrap_or(0) as i64;

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

        // Sin tapa incrustada, la que está al lado del archivo. Es lo que dejan
        // los ripeadores y casi todas las descargas, y es la única fuente que
        // funciona sin conexión **y** sin equivocarse: la puso quien armó la
        // carpeta. Lo que había antes para este caso era salir a buscarla a una
        // API, que necesita red y puede traer la de otro disco.
        if computed_cover_data_url.is_none() {
            if let Some(tapa) = tapa_de_la_carpeta(&canonical_path, tapa_por_carpeta) {
                computed_cover_data_url = Some(tapa.data_url);
                computed_dominant_color = tapa.dominant_color;
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
        album_artist,
        track_no,
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

/// Qué archivos entran a la biblioteca al barrer una carpeta.
///
/// La lista no describe lo que se puede reproducir —de eso se encarga ffmpeg,
/// que decodifica bastante más— sino lo que vale la pena abrir buscando música.
/// El límite de verdad es **`lofty`**: un archivo cuyas etiquetas no se pueden
/// leer no entra a la biblioteca aunque suene, así que sumar una extensión que
/// `lofty` no entiende sólo convierte un archivo que hoy se saltea en silencio
/// en uno que cuenta como fallido.
///
/// Por eso no está `mka`: Matroska sí lo reproduce ffmpeg, pero `lofty` 0.21 no
/// lo reconoce —«No format could be determined»— y el barrido lo contaría como
/// un fallo. Cuando lo soporte, es una palabra más.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "wav", "m4a", "m4b", "aac", "opus", "wma", "aiff", "aif", "ape",
    "wv",
];

pub fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Ayudantes que usan tanto las pruebas de acá como las del barrido.
#[cfg(test)]
pub(crate) mod ayudantes_de_prueba {
    use super::*;
    use lofty::prelude::ItemKey;

    /// Un MP3 de verdad con las etiquetas que pida quien lo llame.
    ///
    /// Ocho tramas y no una: `lofty` no da por bueno un MPEG hasta encontrar
    /// varias cabeceras seguidas, y con una sola la prueba pasaría en verde sin
    /// haber leído nunca una etiqueta.
    pub(crate) fn archivo_etiquetado(
        dir: &Path,
        nombre: &str,
        etiquetas: &[(ItemKey, &str)],
    ) -> PathBuf {
        use lofty::config::WriteOptions;
        use lofty::tag::{Tag, TagExt, TagType};

        let audio = dir.join(nombre);
        let mut bytes = Vec::new();
        for _ in 0..8 {
            // MPEG-1 Layer III, 128 kbps, 44,1 kHz: 417 bytes por trama.
            bytes.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
            bytes.extend(std::iter::repeat(0u8).take(413));
        }
        fs::write(&audio, &bytes).expect("no se pudo escribir el audio");

        let mut tag = Tag::new(TagType::Id3v2);
        for (clave, valor) in etiquetas {
            tag.insert_text(clave.clone(), valor.to_string());
        }
        tag.save_to_path(&audio, WriteOptions::default())
            .expect("no se pudo escribir la etiqueta");

        audio
    }
}

#[cfg(test)]
mod tests {
    use super::ayudantes_de_prueba::archivo_etiquetado;
    use super::*;
    use std::path::PathBuf;

    /// Un PNG de un solo color, armado en memoria.
    fn png_de_un_color(r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut imagen = image::RgbImage::new(24, 24);
        for pixel in imagen.pixels_mut() {
            *pixel = image::Rgb([r, g, b]);
        }
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(imagen)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
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
        assert!(color[1..].chars().all(|c| c.is_ascii_hexdigit()), "{color}");
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
    fn se_lee_el_numero_de_pista_y_el_artista_del_album() {
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(
            dir.path(),
            "03.mp3",
            &[
                (ItemKey::TrackTitle, "Tercera"),
                (ItemKey::TrackArtist, "Quien la canta"),
                (ItemKey::AlbumTitle, "Una recopilación"),
                (ItemKey::AlbumArtist, "Varios artistas"),
                (ItemKey::TrackNumber, "3"),
            ],
        );

        let track = extract_track_from_file(&audio).expect("leer el archivo");

        assert_eq!(track.track_no, 3);
        assert_eq!(track.album_artist, "Varios artistas");
        // Y el artista del tema sigue siendo el suyo, que es todo el punto de
        // que haya dos campos.
        assert_eq!(track.artist, "Quien la canta");
    }

    #[test]
    fn un_archivo_sin_esas_etiquetas_no_inventa_nada() {
        // Lo normal en un disco de un solo artista, y en cualquier archivo
        // suelto: cero y vacío, que es lo que el agrupado entiende como «no lo
        // dice».
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(
            dir.path(),
            "suelto.mp3",
            &[
                (ItemKey::TrackTitle, "Un tema"),
                (ItemKey::TrackArtist, "Alguien"),
            ],
        );

        let track = extract_track_from_file(&audio).expect("leer el archivo");

        assert_eq!(track.track_no, 0);
        assert_eq!(track.album_artist, "");
    }

    #[test]
    fn un_numero_de_pista_con_el_total_se_lee_igual() {
        // «3/12» es como lo escriben la mitad de los etiquetadores.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(
            dir.path(),
            "03.mp3",
            &[
                (ItemKey::TrackTitle, "Tercera"),
                (ItemKey::TrackNumber, "3/12"),
            ],
        );

        assert_eq!(extract_track_from_file(&audio).expect("leer").track_no, 3);
    }

    /// Un MP3 con una tapa incrustada del color que se le pida.
    fn archivo_con_tapa_incrustada(dir: &Path, nombre: &str, color: (u8, u8, u8)) -> PathBuf {
        use lofty::config::WriteOptions;
        use lofty::picture::{MimeType, Picture};
        use lofty::tag::{Tag, TagExt, TagType};

        let audio = archivo_etiquetado(dir, nombre, &[(ItemKey::TrackTitle, "Un tema")]);

        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::TrackTitle, "Un tema".to_string());
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(MimeType::Png),
            None,
            png_de_un_color(color.0, color.1, color.2),
        ));
        tag.save_to_path(&audio, WriteOptions::default())
            .expect("no se pudo escribir la tapa");

        audio
    }

    /// Un archivo de imagen con el nombre que se le pida.
    fn imagen_en(dir: &Path, nombre: &str, color: (u8, u8, u8)) -> PathBuf {
        let path = dir.join(nombre);
        fs::write(&path, png_de_un_color(color.0, color.1, color.2)).expect("escribir la imagen");
        path
    }

    #[test]
    fn se_encuentra_la_tapa_que_esta_al_lado_del_archivo() {
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        let mut cache = TapaPorCarpeta::new();
        let tapa = tapa_de_la_carpeta(&audio, &mut cache).expect("tendría que encontrarla");

        assert!(tapa.data_url.starts_with("data:image/jpeg;base64,"));
        assert!(tapa.dominant_color.is_some());
    }

    #[test]
    fn el_nombre_se_reconoce_con_mayusculas() {
        // `Folder.jpg` con mayúscula es lo que deja medio Windows, y en Linux
        // ése es otro archivo.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        imagen_en(dir.path(), "Folder.JPG", (0x20, 0x20, 0xC0));

        let mut cache = TapaPorCarpeta::new();
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_some());
    }

    #[test]
    fn entre_varias_gana_la_primera_de_la_lista() {
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        // `cover.jpg` va antes que `folder.jpg`, y el color lo delata.
        imagen_en(dir.path(), "folder.jpg", (0x20, 0xC0, 0x20));
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        let mut cache = TapaPorCarpeta::new();
        let tapa = tapa_de_la_carpeta(&audio, &mut cache).expect("tendría que encontrarla");
        let color = tapa.dominant_color.expect("con color");

        let rojo = u8::from_str_radix(&color[1..3], 16).expect("rojo");
        let verde = u8::from_str_radix(&color[3..5], 16).expect("verde");
        assert!(rojo > verde, "la elegida tendría que ser la roja: {color}");
    }

    #[test]
    fn una_imagen_cualquiera_no_es_la_tapa() {
        // Acá es donde empiezan las tapas equivocadas: la foto del grupo, el
        // escaneo del librito. Sólo los nombres de la lista.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        imagen_en(dir.path(), "la-banda-en-vivo.jpg", (0xC0, 0x20, 0x20));

        let mut cache = TapaPorCarpeta::new();
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_none());
    }

    #[test]
    fn una_tapa_enorme_se_ignora() {
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        fs::write(
            dir.path().join("cover.jpg"),
            vec![0u8; (MAXIMO_BYTES_DE_TAPA + 1) as usize],
        )
        .expect("escribir");

        let mut cache = TapaPorCarpeta::new();
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_none());
    }

    #[test]
    fn la_carpeta_se_mira_una_sola_vez() {
        // Un disco de veinte temas comparte una imagen: sin el caché se la leería
        // y decodificaría veinte veces.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        let mut cache = TapaPorCarpeta::new();
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_some());
        assert_eq!(cache.len(), 1);

        // Se borra la imagen: si volviera a mirar el disco, no la encontraría.
        fs::remove_file(dir.path().join("cover.jpg")).expect("borrar");
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_some());
    }

    #[test]
    fn una_carpeta_sin_tapa_tambien_se_recuerda() {
        // El caso negativo importa igual: sin recordarlo, cada tema de una
        // carpeta sin tapa vuelve a recorrerla entera.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);

        let mut cache = TapaPorCarpeta::new();
        assert!(tapa_de_la_carpeta(&audio, &mut cache).is_none());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn la_tapa_incrustada_le_gana_a_la_de_la_carpeta() {
        // Quien etiquetó el archivo decidió cuál es la tapa de ese tema; la de
        // la carpeta es para cuando no hay ninguna.
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_con_tapa_incrustada(dir.path(), "tema.mp3", (0x20, 0xC0, 0x20));
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        let metadata = extract_now_playing_metadata(&audio).expect("leer");
        let color = metadata.dominant_color.expect("con color");

        let rojo = u8::from_str_radix(&color[1..3], 16).expect("rojo");
        let verde = u8::from_str_radix(&color[3..5], 16).expect("verde");
        assert!(
            verde > rojo,
            "tendría que ganar la verde incrustada: {color}"
        );
    }

    #[test]
    fn sin_tapa_incrustada_se_usa_la_de_la_carpeta() {
        let dir = tempfile::tempdir().expect("temp dir");
        let audio = archivo_etiquetado(dir.path(), "tema.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        let metadata = extract_now_playing_metadata(&audio).expect("leer");

        assert!(metadata.cover_data_url.is_some());
    }

    #[test]
    fn la_tapa_se_comparte_entre_llamadas_al_comando() {
        // La ventana pide los datos de cada tema por separado: sin el caché
        // compartido, un disco de veinte temas leería veinte veces la misma
        // imagen.
        let dir = tempfile::tempdir().expect("temp dir");
        let primero = archivo_etiquetado(dir.path(), "01.mp3", &[(ItemKey::TrackTitle, "Una")]);
        let segundo = archivo_etiquetado(dir.path(), "02.mp3", &[(ItemKey::TrackTitle, "Otra")]);
        imagen_en(dir.path(), "cover.jpg", (0xC0, 0x20, 0x20));

        assert!(extract_now_playing_metadata(&primero)
            .expect("leer")
            .cover_data_url
            .is_some());

        // Se borra la imagen: si el segundo volviera a mirar el disco, no la
        // encontraría.
        fs::remove_file(dir.path().join("cover.jpg")).expect("borrar");

        assert!(extract_now_playing_metadata(&segundo)
            .expect("leer")
            .cover_data_url
            .is_some());
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

    #[test]
    fn aiff_se_reconoce_con_las_dos_extensiones() {
        // Dos archivos idénticos se indexaban o no según cómo estuviera escrito
        // el nombre: `aiff` estaba en la lista y `aif` —como lo escribe medio
        // mundo— no.
        assert!(is_supported_audio_file(Path::new("/m/tema.aiff")));
        assert!(is_supported_audio_file(Path::new("/m/tema.aif")));
    }

    #[test]
    fn entran_los_formatos_sin_perdida_que_faltaban() {
        // Monkey's Audio y WavPack son comunes en colecciones ripeadas, y
        // `lofty` lee las etiquetas de los dos.
        assert!(is_supported_audio_file(Path::new("/m/tema.ape")));
        assert!(is_supported_audio_file(Path::new("/m/tema.wv")));
    }

    #[test]
    fn el_audiolibro_es_el_mismo_contenedor_que_el_m4a() {
        assert!(is_supported_audio_file(Path::new("/m/libro.m4b")));
    }

    #[test]
    fn alac_no_es_una_extension() {
        // ALAC es un códec, no un contenedor: un archivo ALAC se llama `.m4a`
        // —que ya estaba— o `.caf`. La entrada no hacía nada salvo delatar que
        // la lista se armó de memoria.
        assert!(!is_supported_audio_file(Path::new("/m/tema.alac")));
    }

    #[test]
    fn matroska_queda_afuera_a_proposito() {
        // ffmpeg lo reproduce, pero `lofty` 0.21 no reconoce el formato, así que
        // el archivo entraría al barrido sólo para contarse como fallido. Si un
        // día `lofty` lo soporta, esto es lo que hay que dar vuelta.
        assert!(!is_supported_audio_file(Path::new("/m/tema.mka")));
    }
}
