use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::Emitter;
use walkdir::WalkDir;

use crate::audio::{extract_track_from_file, is_supported_audio_file, SCAN_VERSION};
use crate::db::{
    get_database_path, get_setting, index_track, known_mtimes_under, open_database, remove_tracks,
    set_setting,
};
use crate::structs::ScanSummary;

/// Dónde queda anotada la versión de lectura con la que se indexó la biblioteca.
const SCAN_VERSION_SETTING: &str = "scan_version";

/// La fecha de modificación de un archivo, en segundos desde la época.
///
/// `None` cuando el sistema no la da. Ahí el archivo se lee siempre, que es lo
/// seguro: sin fecha no hay forma de saber si cambió.
fn mtime_de(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|desde_la_epoca| desde_la_epoca.as_secs() as i64)
}

fn scan_folders_internal(folders: &[String]) -> Result<ScanSummary, String> {
    let db_path = get_database_path()?;
    let mut conn = open_database(&db_path)?;

    scan_folders_into(&mut conn, folders)
}

/// El barrido, contra la base que se le pase.
///
/// Separado del comando para poder probarlo con una base temporal en vez de la
/// de quien usa el equipo.
fn scan_folders_into(
    conn: &mut rusqlite::Connection,
    folders: &[String],
) -> Result<ScanSummary, String> {
    let mut summary = ScanSummary {
        scanned_files: 0,
        inserted_tracks: 0,
        updated_tracks: 0,
        unchanged_tracks: 0,
        removed_tracks: 0,
        skipped_non_audio: 0,
        failed_files: 0,
    };

    // Cuando cambian las reglas con las que se leen las etiquetas, lo que está
    // indexado quedó leído con las de antes. Subir la versión fuerza una
    // relectura completa, una sola vez, en vez de necesitar un apaño por cada
    // cambio — que es lo que se hizo la última vez, al empezar a leer el número
    // de pista.
    let releer_todo = get_setting(conn, SCAN_VERSION_SETTING).as_deref() != Some(SCAN_VERSION);

    for folder in folders {
        let root = PathBuf::from(folder);
        if !root.exists() || !root.is_dir() {
            continue;
        }

        let raiz = root.to_string_lossy().to_string();
        let conocidos: HashMap<String, i64> = known_mtimes_under(conn, &raiz)?;
        let mut vistos = HashSet::<String>::new();
        // Un error al recorrer —un subdirectorio que dejó de poder leerse, un
        // punto de montaje a medio montar— hace que los archivos que hay abajo
        // no aparezcan. Eso no es que se hayan borrado, así que si pasa, esta
        // carpeta no da de baja nada.
        let mut hubo_errores_al_recorrer = false;

        for entrada in WalkDir::new(&root).follow_links(false) {
            let entrada = match entrada {
                Ok(entrada) => entrada,
                Err(_) => {
                    hubo_errores_al_recorrer = true;
                    continue;
                }
            };

            let path = entrada.path();
            if !path.is_file() {
                continue;
            }

            summary.scanned_files += 1;

            if !is_supported_audio_file(path) {
                summary.skipped_non_audio += 1;
                continue;
            }

            // Se canoniza antes de comparar porque es lo que guarda la fila:
            // si acá quedara la ruta sin canonizar, un enlace o un `..` en el
            // camino haría ver como nuevo un archivo que ya estaba.
            let canonica = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            let clave = canonica.to_string_lossy().to_string();
            vistos.insert(clave.clone());

            let mtime = entrada.metadata().ok().and_then(|m| mtime_de(&m));
            let conocido = conocidos.get(&clave).copied();

            // Lo caro es abrir el archivo y parsearle las etiquetas. Si la
            // fecha es la misma que la guardada, no hace falta: el barrido pasa
            // a ser un `stat` por archivo.
            let sin_cambios = !releer_todo
                && match (mtime, conocido) {
                    (Some(actual), Some(guardado)) => guardado != 0 && actual == guardado,
                    _ => false,
                };

            if sin_cambios {
                summary.unchanged_tracks += 1;
                continue;
            }

            match extract_track_from_file(path) {
                Ok(track) => {
                    index_track(conn, &track, mtime.unwrap_or(0))?;
                    if conocido.is_some() {
                        summary.updated_tracks += 1;
                    } else {
                        summary.inserted_tracks += 1;
                    }
                }
                Err(_) => {
                    summary.failed_files += 1;
                }
            }
        }

        if hubo_errores_al_recorrer {
            continue;
        }

        let desaparecidos: Vec<String> = conocidos
            .keys()
            .filter(|path| !vistos.contains(*path))
            .cloned()
            .collect();

        summary.removed_tracks += remove_tracks(conn, &desaparecidos)?;
    }

    if releer_todo {
        set_setting(conn, SCAN_VERSION_SETTING, SCAN_VERSION)?;
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

/// Walks the folders on a worker thread.
///
/// Scanning reads and decodes the tags of every audio file it finds — minutes
/// of work on a real library. Running it on the main thread left the window
/// frozen and unrepainted for the whole scan.
async fn scan_off_main_thread(folders: Vec<String>) -> Result<ScanSummary, String> {
    tauri::async_runtime::spawn_blocking(move || scan_folders_internal(&folders))
        .await
        .map_err(|e| format!("El escaneo falló: {e}"))?
}

#[tauri::command]
pub async fn scan_music_folders(folders: Vec<String>) -> Result<ScanSummary, String> {
    scan_off_main_thread(folders).await
}

#[tauri::command]
pub async fn scan_default_music_folder(
    app_handle: tauri::AppHandle,
) -> Result<ScanSummary, String> {
    let folders = resolve_default_music_folders();
    let result = scan_off_main_thread(folders).await?;

    // Emit scan complete event
    let _ = app_handle.emit("scan-complete", &result);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use lofty::prelude::ItemKey;
    use rusqlite::Connection;

    use super::*;
    use crate::audio::ayudantes_de_prueba::archivo_etiquetado;
    use crate::db::{list_tracks, open_database};

    /// Una biblioteca vacía en un directorio temporal, y la carpeta de música.
    fn biblioteca() -> (tempfile::TempDir, tempfile::TempDir, Connection) {
        let base = tempfile::tempdir().expect("temp dir");
        let musica = tempfile::tempdir().expect("temp dir");
        let conn = open_database(&base.path().join("resonance.db")).expect("abrir");
        (base, musica, conn)
    }

    /// Le fija la fecha de modificación a un archivo, para simular que cambió
    /// —o que no— sin depender del reloj.
    fn fechar(path: &Path, segundos_desde_la_epoca: u64) {
        let archivo = std::fs::File::options()
            .write(true)
            .open(path)
            .expect("abrir para fechar");
        archivo
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(segundos_desde_la_epoca))
            .expect("fechar");
    }

    fn barrer(conn: &mut Connection, musica: &tempfile::TempDir) -> ScanSummary {
        scan_folders_into(conn, &[musica.path().to_string_lossy().to_string()]).expect("barrer")
    }

    #[test]
    fn un_archivo_nuevo_se_indexa() {
        let (_base, musica, mut conn) = biblioteca();
        archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.inserted_tracks, 1);
        assert_eq!(list_tracks(&conn).expect("list").len(), 1);
    }

    #[test]
    fn un_archivo_que_no_cambio_no_se_vuelve_a_abrir() {
        // Es el punto del barrido incremental. Se comprueba rompiendo el
        // archivo después del primer barrido pero dejándole la misma fecha: si
        // el segundo barrido lo abriera, no podría leerlo y contaría un fallo.
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        fechar(&audio, 1_700_000_000);
        barrer(&mut conn, &musica);

        std::fs::write(&audio, b"esto ya no es un mp3").expect("romper");
        fechar(&audio, 1_700_000_000);

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.unchanged_tracks, 1);
        assert_eq!(resumen.failed_files, 0, "no tendría que haberlo abierto");
        assert_eq!(resumen.updated_tracks, 0);
    }

    #[test]
    fn un_archivo_editado_se_relee() {
        // Lo que no pasaba nunca: corregir una etiqueta y que se vea.
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(
            musica.path(),
            "a.mp3",
            &[(ItemKey::TrackTitle, "Titulo con error")],
        );
        fechar(&audio, 1_700_000_000);
        barrer(&mut conn, &musica);

        archivo_etiquetado(
            musica.path(),
            "a.mp3",
            &[(ItemKey::TrackTitle, "Título corregido")],
        );
        fechar(&audio, 1_700_000_900);

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.updated_tracks, 1);
        assert_eq!(resumen.inserted_tracks, 0);
        assert_eq!(
            list_tracks(&conn).expect("list")[0].title,
            "Título corregido"
        );
    }

    #[test]
    fn un_archivo_borrado_se_da_de_baja() {
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Se va")]);
        archivo_etiquetado(musica.path(), "b.mp3", &[(ItemKey::TrackTitle, "Se queda")]);
        barrer(&mut conn, &musica);

        std::fs::remove_file(&audio).expect("borrar");

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.removed_tracks, 1);
        let quedan = list_tracks(&conn).expect("list");
        assert_eq!(quedan.len(), 1);
        assert_eq!(quedan[0].title, "Se queda");
    }

    #[test]
    fn la_baja_no_toca_lo_que_esta_fuera_de_la_carpeta_barrida() {
        // Un tema puede haber entrado por arrastrar y soltar desde cualquier
        // lado. Barrer la carpeta de música no puede llevárselo puesto.
        let (_base, musica, mut conn) = biblioteca();
        let otra = tempfile::tempdir().expect("temp dir");
        let ajeno = archivo_etiquetado(otra.path(), "ajeno.mp3", &[(ItemKey::TrackTitle, "Ajeno")]);
        scan_folders_into(&mut conn, &[otra.path().to_string_lossy().to_string()]).expect("barrer");
        archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Propio")]);

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.removed_tracks, 0);
        assert_eq!(list_tracks(&conn).expect("list").len(), 2);
        assert!(ajeno.exists());
    }

    #[test]
    fn subir_la_version_de_lectura_relee_todo() {
        // Sin esto, cada cambio en cómo se leen las etiquetas necesita su
        // propio apaño para que la biblioteca ya indexada se entere.
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        fechar(&audio, 1_700_000_000);
        barrer(&mut conn, &musica);

        // Lo que hace subir la constante: la versión anotada deja de coincidir.
        set_setting(&conn, SCAN_VERSION_SETTING, "version-vieja").expect("anotar");

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.updated_tracks, 1, "tendría que haberlo releído");
        assert_eq!(resumen.unchanged_tracks, 0);
        assert_eq!(
            get_setting(&conn, SCAN_VERSION_SETTING).as_deref(),
            Some(SCAN_VERSION),
            "y dejar anotada la versión nueva"
        );
    }

    #[test]
    fn una_carpeta_que_no_existe_no_da_de_baja_nada() {
        // Una unidad de red desmontada no es una biblioteca vacía.
        let (_base, musica, mut conn) = biblioteca();
        archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        barrer(&mut conn, &musica);

        let resumen =
            scan_folders_into(&mut conn, &["/carpeta/que/no/existe".to_string()]).expect("barrer");

        assert_eq!(resumen.removed_tracks, 0);
        assert_eq!(list_tracks(&conn).expect("list").len(), 1);
    }

    #[test]
    fn lo_que_no_es_audio_no_cuenta_como_tema() {
        let (_base, musica, mut conn) = biblioteca();
        archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        std::fs::write(musica.path().join("cover.jpg"), b"no soy audio").expect("escribir");

        let resumen = barrer(&mut conn, &musica);

        assert_eq!(resumen.inserted_tracks, 1);
        assert_eq!(resumen.skipped_non_audio, 1);
    }
}
