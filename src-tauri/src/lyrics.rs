use reqwest::Client;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use lofty::file::TaggedFileExt;
use lofty::prelude::ItemKey;
use lofty::probe::Probe;

use crate::structs::{LyricsLine, TrackLyricsPayload};

const LRCLIB_BASE_URL: &str = "https://lrclib.net";

static LYRICS_CACHE: OnceLock<Mutex<HashMap<String, TrackLyricsPayload>>> = OnceLock::new();
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

const NEW_LYRICS_CACHE_RELATIVE_PATH: &str = ".config/resonance/lyrics-cache.json";
const LEGACY_LYRICS_CACHE_RELATIVE_PATH: &str = ".config/vasak/lyrics-cache.json";

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LrcLibRecord {
    id: u64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricsQuery {
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration_seconds: u64,
}

fn cache() -> &'static Mutex<HashMap<String, TrackLyricsPayload>> {
    LYRICS_CACHE.get_or_init(|| Mutex::new(load_persistent_cache()))
}

fn lyrics_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)?;
    Some(home.join(NEW_LYRICS_CACHE_RELATIVE_PATH))
}

fn legacy_lyrics_cache_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)?;
    Some(home.join(LEGACY_LYRICS_CACHE_RELATIVE_PATH))
}

fn load_persistent_cache() -> HashMap<String, TrackLyricsPayload> {
    let Some(new_path) = lyrics_cache_path() else {
        return HashMap::new();
    };

    if let Ok(content) = fs::read_to_string(&new_path) {
        return serde_json::from_str::<HashMap<String, TrackLyricsPayload>>(&content)
            .unwrap_or_default();
    }

    let Some(legacy_path) = legacy_lyrics_cache_path() else {
        return HashMap::new();
    };

    let content = match fs::read_to_string(&legacy_path) {
        Ok(content) => content,
        Err(_) => return HashMap::new(),
    };

    let legacy_cache =
        serde_json::from_str::<HashMap<String, TrackLyricsPayload>>(&content).unwrap_or_default();

    if !legacy_cache.is_empty() {
        persist_cache(&legacy_cache);
    }

    legacy_cache
}

fn persist_cache(cache_map: &HashMap<String, TrackLyricsPayload>) {
    let Some(path) = lyrics_cache_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[lyrics] no se pudo crear carpeta de caché local: {}", err);
            return;
        }
    }

    let content = match serde_json::to_string(cache_map) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[lyrics] no se pudo serializar caché local: {}", err);
            return;
        }
    };

    if let Err(err) = fs::write(path, content) {
        eprintln!("[lyrics] no se pudo guardar caché local: {}", err);
    }
}

fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(6))
            .connect_timeout(Duration::from_secs(3))
            .user_agent("VasakResonance/0.1 (+https://github.com/vasak-group)")
            .build()
            .expect("failed to create reqwest client")
    })
}

fn normalize_for_query(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("unknown artist")
        || trimmed.eq_ignore_ascii_case("unknown album")
        || trimmed.eq_ignore_ascii_case("unknown title")
    {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn query_key(query: &LyricsQuery) -> String {
    format!(
        "{}|{}|{}|{}",
        query.track_name.to_lowercase(),
        query.artist_name.to_lowercase(),
        query.album_name.to_lowercase(),
        query.duration_seconds
    )
}

fn parse_timestamp_to_ms(token: &str) -> Option<u64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }

    let minutes = parts[0].parse::<u64>().ok()?;
    let sec_parts = parts[1].split('.').collect::<Vec<_>>();
    let seconds = sec_parts.first()?.parse::<u64>().ok()?;

    let fractional_ms = match sec_parts.get(1) {
        Some(frac) if !frac.is_empty() => {
            if frac.len() >= 3 {
                frac[0..3].parse::<u64>().ok()?
            } else if frac.len() == 2 {
                frac.parse::<u64>().ok()? * 10
            } else {
                frac.parse::<u64>().ok()? * 100
            }
        }
        _ => 0,
    };

    Some(minutes * 60_000 + seconds * 1_000 + fractional_ms)
}

pub fn parse_lrc_lines(lrc_text: &str) -> Vec<LyricsLine> {
    let mut lines = Vec::<LyricsLine>::new();

    for raw_line in lrc_text.lines() {
        let mut rest = raw_line.trim();
        if rest.is_empty() {
            continue;
        }

        let mut stamps = Vec::<u64>::new();
        loop {
            if !rest.starts_with('[') {
                break;
            }
            let Some(end) = rest.find(']') else {
                break;
            };
            let token = &rest[1..end];
            if let Some(ms) = parse_timestamp_to_ms(token) {
                stamps.push(ms);
            }
            rest = rest[end + 1..].trim_start();
        }

        if stamps.is_empty() {
            continue;
        }

        let content = rest.to_string();
        for ms in stamps {
            lines.push(LyricsLine {
                time_ms: ms,
                text: content.clone(),
            });
        }
    }

    lines.sort_by(|a, b| match a.time_ms.cmp(&b.time_ms) {
        Ordering::Equal => a.text.cmp(&b.text),
        ordering => ordering,
    });
    lines
}

fn record_to_payload(source: &str, record: &LrcLibRecord) -> TrackLyricsPayload {
    let synced_lyrics = record
        .synced_lyrics
        .as_ref()
        .map(|lyrics| lyrics.trim().to_string())
        .filter(|lyrics| !lyrics.is_empty());
    let plain_lyrics = record
        .plain_lyrics
        .as_ref()
        .map(|lyrics| lyrics.trim().to_string())
        .filter(|lyrics| !lyrics.is_empty());

    let lines = synced_lyrics
        .as_ref()
        .map(|lrc| parse_lrc_lines(lrc))
        .unwrap_or_default();

    TrackLyricsPayload {
        source: format!(
            "{}:{}:{}-{}:{}",
            source, record.id, record.artist_name, record.track_name, record.album_name
        ),
        synced: !lines.is_empty(),
        instrumental: record.instrumental,
        plain_lyrics,
        synced_lyrics,
        lines,
    }
}

async fn fetch_signature_record(
    client: &Client,
    endpoint: &str,
    query: &LyricsQuery,
) -> Option<LrcLibRecord> {
    let duration = query.duration_seconds;
    let url = format!("{}/{}", LRCLIB_BASE_URL, endpoint);

    let response = match client
        .get(url.clone())
        .query(&[
            ("track_name", &query.track_name),
            ("artist_name", &query.artist_name),
            ("album_name", &query.album_name),
            ("duration", &duration.to_string()),
        ])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("[lyrics] request error for {}: {}", url, err);
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        eprintln!("[lyrics] non-success {} {} -> {}", url, status, body);
        return None;
    }

    match response.json::<LrcLibRecord>().await {
        Ok(rec) => Some(rec),
        Err(err) => {
            eprintln!("[lyrics] failed to parse JSON from {}: {}", url, err);
            None
        }
    }
}

async fn fetch_search_record(client: &Client, query: &LyricsQuery) -> Option<LrcLibRecord> {
    let base = format!("{} {}", query.artist_name, query.track_name)
        .trim()
        .to_string();
    if base.is_empty() {
        return None;
    }
    let url = format!("{}/api/search", LRCLIB_BASE_URL);

    let response = match client
        .get(url.clone())
        .query(&[("q", base.as_str())])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            eprintln!("[lyrics] search request error for {}: {}", url, err);
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        eprintln!("[lyrics] search non-success {} {} -> {}", url, status, body);
        return None;
    }

    let mut items = match response.json::<Vec<LrcLibRecord>>().await {
        Ok(list) => list,
        Err(err) => {
            eprintln!("[lyrics] failed to parse search JSON: {}", err);
            return None;
        }
    };

    if items.is_empty() {
        eprintln!("[lyrics] search returned empty list for q='{}'", base);
        return None;
    }

    items.sort_by(|a, b| {
        let da = (a.duration.round() as i64 - query.duration_seconds as i64).abs();
        let db = (b.duration.round() as i64 - query.duration_seconds as i64).abs();
        da.cmp(&db)
    });

    items.into_iter().next()
}

/// Techo de lo que se lee de un archivo de letras.
///
/// Una letra son unos pocos kilobytes. El techo no es por las letras sino por
/// lo que puede haber quedado con ese nombre al lado de la música: se prefiere
/// ignorarlo a cargarlo entero en memoria.
const MAXIMO_BYTES_DE_LETRA: u64 = 1024 * 1024;

/// Lee un archivo de letras que esté al lado del audio.
///
/// Vacío —o ausente, o ilegible, o más grande que el techo— es lo mismo que no
/// estar: se sigue con la fuente siguiente.
fn leer_archivo_de_letras(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAXIMO_BYTES_DE_LETRA {
        return None;
    }

    // Un archivo que no sea UTF-8 —los `.lrc` viejos suelen estar en la
    // codificación de su país— se deja pasar en vez de adivinar: la letra sale
    // de la red, como antes.
    let contenido = fs::read_to_string(path).ok()?;
    let contenido = contenido.trim().to_string();

    if contenido.is_empty() {
        None
    } else {
        Some(contenido)
    }
}

/// Arma la letra sincronizada, si el texto tiene marcas de tiempo.
fn como_sincronizada(fuente: &str, texto: &str) -> Option<TrackLyricsPayload> {
    let lines = parse_lrc_lines(texto);
    if lines.is_empty() {
        return None;
    }

    Some(TrackLyricsPayload {
        source: fuente.to_string(),
        synced: true,
        instrumental: false,
        plain_lyrics: None,
        synced_lyrics: Some(texto.to_string()),
        lines,
    })
}

/// Arma la letra plana: la que no tiene tiempos y se muestra entera.
fn como_plana(fuente: &str, texto: &str) -> TrackLyricsPayload {
    TrackLyricsPayload {
        source: fuente.to_string(),
        synced: false,
        instrumental: false,
        plain_lyrics: Some(texto.to_string()),
        synced_lyrics: None,
        lines: Vec::new(),
    }
}

/// La letra guardada dentro del propio archivo de audio.
fn letra_de_la_etiqueta(audio: &Path) -> Option<String> {
    let tagged = Probe::open(audio).ok()?.read().ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let texto = tag.get_string(&ItemKey::Lyrics)?.trim().to_string();

    if texto.is_empty() {
        None
    } else {
        Some(texto)
    }
}

/// La letra que ya está en el disco, si está.
///
/// Se mira antes que LRCLIB por dos motivos. El primero es que funciona sin
/// conexión, que es el estado normal de una biblioteca local. El segundo es que
/// quien se tomó el trabajo de dejar un `.lrc` al lado del archivo quiere **ese**
/// —puede ser la versión en vivo, la traducida, o la que corrige lo que el
/// servicio tiene mal—, y una letra de la red no tiene por qué ganarle.
///
/// El orden es sincronizada primero: entre un `.lrc` con tiempos y un `.txt`
/// plano, el que sirve para seguir la canción es el primero.
pub fn letras_del_disco(audio: &Path) -> Option<TrackLyricsPayload> {
    // 1. El `.lrc` de al lado. Si no tiene ni una marca de tiempo no es un LRC
    //    de verdad, y se lo trata como lo que quedó: texto plano.
    if let Some(texto) = leer_archivo_de_letras(&audio.with_extension("lrc")) {
        return Some(
            como_sincronizada("archivo:lrc", &texto)
                .unwrap_or_else(|| como_plana("archivo:lrc", &texto)),
        );
    }

    // 2. La etiqueta del archivo, que es donde la dejan los etiquetadores. A
    //    veces trae los tiempos adentro, así que se prueba igual.
    if let Some(texto) = letra_de_la_etiqueta(audio) {
        return Some(
            como_sincronizada("etiqueta", &texto).unwrap_or_else(|| como_plana("etiqueta", &texto)),
        );
    }

    // 3. Un `.txt` con el mismo nombre. Nunca tiene tiempos.
    leer_archivo_de_letras(&audio.with_extension("txt"))
        .map(|texto| como_plana("archivo:txt", &texto))
}

pub async fn fetch_track_lyrics(query: LyricsQuery) -> Result<TrackLyricsPayload, String> {
    let normalized = LyricsQuery {
        track_name: normalize_for_query(&query.track_name),
        artist_name: normalize_for_query(&query.artist_name),
        album_name: normalize_for_query(&query.album_name),
        duration_seconds: query.duration_seconds,
    };

    let key = query_key(&normalized);
    if let Ok(locked) = cache().lock() {
        if let Some(cached) = locked.get(&key) {
            return Ok(cached.clone());
        }
    }

    let client = http_client();

    let mut record = fetch_signature_record(client, "api/get", &normalized).await;
    if record.is_none() {
        record = fetch_search_record(client, &normalized).await;
    }

    let payload = match record {
        Some(record) => {
            if record.synced_lyrics.as_deref().unwrap_or("").is_empty()
                && record.plain_lyrics.as_deref().unwrap_or("").is_empty()
            {
                return Err("No se encontraron letras para esta canción".to_string());
            }

            record_to_payload("lrclib", &record)
        }
        None => return Err("No se encontraron letras para esta canción".to_string()),
    };

    if let Ok(mut locked) = cache().lock() {
        locked.insert(key, payload.clone());
        persist_cache(&locked);
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un directorio con un archivo de audio de mentira y lo que se le quiera
    /// poner al lado. El audio no necesita ser audio de verdad: mientras haya un
    /// `.lrc` o un `.txt`, nunca se lo abre; y cuando se lo abre —para mirarle la
    /// etiqueta— fallar es una de las respuestas previstas.
    fn carpeta_con(archivos: &[(&str, &str)]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = dir.path().join("tema.mp3");
        fs::write(&audio, b"esto no es un mp3").expect("no se pudo escribir el audio");

        for (nombre, contenido) in archivos {
            fs::write(dir.path().join(nombre), contenido).expect("no se pudo escribir");
        }

        (dir, audio)
    }

    const LRC: &str = "[00:12.50]Primera línea\n[00:18.00]Segunda línea\n";

    /// Un MP3 de verdad, con la letra escrita en la etiqueta.
    ///
    /// Ocho tramas y no una: `lofty` no da por bueno un MPEG hasta encontrar
    /// varias cabeceras seguidas, y con una sola la prueba habría pasado en
    /// verde sin haber leído nunca una etiqueta.
    fn audio_con_letra_en_la_etiqueta(dir: &Path, nombre: &str, letra: &str) -> std::path::PathBuf {
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
        tag.insert_text(ItemKey::Lyrics, letra.to_string());
        tag.save_to_path(&audio, WriteOptions::default())
            .expect("no se pudo escribir la etiqueta");

        audio
    }

    #[test]
    fn el_lrc_de_al_lado_da_letra_sincronizada() {
        let (_dir, audio) = carpeta_con(&[("tema.lrc", LRC)]);

        let letras = letras_del_disco(&audio).expect("tendría que encontrar el .lrc");

        assert!(letras.synced);
        assert_eq!(letras.source, "archivo:lrc");
        assert_eq!(letras.lines.len(), 2);
        assert_eq!(letras.lines[0].time_ms, 12_500);
        assert_eq!(letras.lines[0].text, "Primera línea");
    }

    #[test]
    fn un_lrc_sin_tiempos_es_texto_plano() {
        // Pasa: alguien guarda la letra con la extensión del formato
        // sincronizado. Sirve igual, sólo que sin seguir la canción.
        let (_dir, audio) = carpeta_con(&[("tema.lrc", "Primera línea\nSegunda línea\n")]);

        let letras = letras_del_disco(&audio).expect("tendría que usarlo igual");

        assert!(!letras.synced);
        assert!(letras.lines.is_empty());
        assert_eq!(
            letras.plain_lyrics.as_deref(),
            Some("Primera línea\nSegunda línea")
        );
    }

    #[test]
    fn sin_lrc_vale_el_txt() {
        let (_dir, audio) = carpeta_con(&[("tema.txt", "Una letra sin tiempos\n")]);

        let letras = letras_del_disco(&audio).expect("tendría que encontrar el .txt");

        assert!(!letras.synced);
        assert_eq!(letras.source, "archivo:txt");
        assert_eq!(
            letras.plain_lyrics.as_deref(),
            Some("Una letra sin tiempos")
        );
    }

    #[test]
    fn el_lrc_le_gana_al_txt() {
        // Entre los dos gana el que sirve para seguir la canción.
        let (_dir, audio) = carpeta_con(&[("tema.lrc", LRC), ("tema.txt", "La otra letra")]);

        let letras = letras_del_disco(&audio).expect("tendría que haber letra");

        assert_eq!(letras.source, "archivo:lrc");
    }

    #[test]
    fn un_archivo_vacio_es_lo_mismo_que_no_estar() {
        let (_dir, audio) = carpeta_con(&[("tema.lrc", "   \n\n"), ("tema.txt", "La que sirve")]);

        let letras = letras_del_disco(&audio).expect("tendría que caer en el .txt");

        assert_eq!(letras.source, "archivo:txt");
    }

    #[test]
    fn un_archivo_enorme_se_ignora() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = dir.path().join("tema.mp3");
        fs::write(&audio, b"esto no es un mp3").expect("no se pudo escribir el audio");
        fs::write(
            dir.path().join("tema.lrc"),
            "x".repeat((MAXIMO_BYTES_DE_LETRA + 1) as usize),
        )
        .expect("no se pudo escribir");

        assert!(letras_del_disco(&audio).is_none());
    }

    #[test]
    fn sin_nada_al_lado_no_hay_letra_del_disco() {
        // Y ahí es donde sigue entrando LRCLIB, que es lo que hacía siempre.
        let (_dir, audio) = carpeta_con(&[]);

        assert!(letras_del_disco(&audio).is_none());
    }

    #[test]
    fn el_archivo_de_letras_sigue_al_nombre_del_audio() {
        // `tema.parte1.mp3` busca `tema.parte1.lrc`, no `tema.lrc`.
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = dir.path().join("tema.parte1.mp3");
        fs::write(&audio, b"esto no es un mp3").expect("no se pudo escribir el audio");
        fs::write(dir.path().join("tema.parte1.lrc"), LRC).expect("no se pudo escribir");
        fs::write(dir.path().join("tema.lrc"), "La de otro tema").expect("no se pudo escribir");

        let letras = letras_del_disco(&audio).expect("tendría que encontrar el suyo");

        assert_eq!(letras.lines.len(), 2);
    }

    #[test]
    fn la_letra_de_la_etiqueta_se_usa_cuando_no_hay_archivo_al_lado() {
        // Es donde la dejan los etiquetadores y casi todas las descargas.
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = audio_con_letra_en_la_etiqueta(dir.path(), "tema.mp3", "Desde la etiqueta");

        let letras = letras_del_disco(&audio).expect("tendría que leer la etiqueta");

        assert_eq!(letras.source, "etiqueta");
        assert!(!letras.synced);
        assert_eq!(letras.plain_lyrics.as_deref(), Some("Desde la etiqueta"));
    }

    #[test]
    fn una_etiqueta_con_tiempos_sale_sincronizada() {
        // Pasa más de lo que parece: el formato sincronizado entra tal cual en
        // el campo de la letra.
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = audio_con_letra_en_la_etiqueta(dir.path(), "tema.mp3", LRC);

        let letras = letras_del_disco(&audio).expect("tendría que leer la etiqueta");

        assert_eq!(letras.source, "etiqueta");
        assert!(letras.synced);
        assert_eq!(letras.lines.len(), 2);
        assert_eq!(letras.lines[1].time_ms, 18_000);
    }

    #[test]
    fn el_archivo_de_al_lado_le_gana_a_la_etiqueta() {
        // Quien dejó un `.lrc` al lado quiere ése: puede ser el que corrige lo
        // que la etiqueta tiene mal.
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = audio_con_letra_en_la_etiqueta(dir.path(), "tema.mp3", "La de la etiqueta");
        fs::write(dir.path().join("tema.lrc"), LRC).expect("no se pudo escribir");

        let letras = letras_del_disco(&audio).expect("tendría que haber letra");

        assert_eq!(letras.source, "archivo:lrc");
    }

    #[test]
    fn una_carpeta_con_ese_nombre_no_es_una_letra() {
        let dir = tempfile::tempdir().expect("no se pudo crear el directorio temporal");
        let audio = dir.path().join("tema.mp3");
        fs::write(&audio, b"esto no es un mp3").expect("no se pudo escribir el audio");
        fs::create_dir(dir.path().join("tema.lrc")).expect("no se pudo crear la carpeta");

        assert!(letras_del_disco(&audio).is_none());
    }
}
