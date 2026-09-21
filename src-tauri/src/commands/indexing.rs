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

    // Una carpeta que no se puede mirar deja la biblioteca a medio releer, así
    // que la versión de lectura no se anota: la próxima vez hay que volver a
    // intentarlo.
    let mut la_pasada_quedo_completa = true;

    for folder in folders {
        let sin_canonizar = PathBuf::from(folder);
        if !sin_canonizar.exists() || !sin_canonizar.is_dir() {
            la_pasada_quedo_completa = false;
            continue;
        }

        // Canonizada, que es como están guardadas las rutas: si la carpeta
        // configurada es un enlace —`~/Música` apuntando a otro disco— o trae un
        // `..`, ninguna fila guardada empieza por ella y la biblioteca parecería
        // vacía. Con eso, cada barrido releería todo y nada se daría de baja.
        let root = std::fs::canonicalize(&sin_canonizar).unwrap_or(sin_canonizar);
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
                    la_pasada_quedo_completa = false;
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

            // Del archivo apuntado y no de la entrada: el recorrido no sigue
            // enlaces, así que para un enlace la entrada describe el enlace. La
            // fila se guarda con la ruta del archivo de verdad, y si la fecha
            // fuera la del enlace, editar el archivo no se notaría nunca.
            let mtime = std::fs::metadata(&canonica).ok().and_then(|m| mtime_de(&m));
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
                    // El archivo está pero no se lo pudo leer. Si la fila
                    // conserva su fecha, el barrido siguiente la da por buena
                    // sin abrirlo y se queda con lo que decía antes, para
                    // siempre. Olvidarle la fecha hace que se reintente.
                    if conocido.is_some() {
                        crate::db::forget_mtime(conn, &clave)?;
                    }
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

    // La versión se anota sólo si la pasada llegó a mirar todo. Anotarla con una
    // carpeta sin montar o un directorio ilegible de por medio dejaría filas
    // leídas con las reglas viejas y el barrido siguiente ya no las tocaría.
    if releer_todo && la_pasada_quedo_completa {
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
    use crate::db::{known_mtimes_under, list_tracks, open_database};

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
    fn una_carpeta_que_es_un_enlace_reconoce_lo_que_ya_estaba() {
        // `~/Música` apuntando a otro disco es de lo más normal. Las filas se
        // guardan con la ruta canónica, así que si la carpeta no se canoniza
        // antes de consultar, la biblioteca parece vacía: todo se relee y nada
        // se da de baja.
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        fechar(&audio, 1_700_000_000);

        let enlaces = tempfile::tempdir().expect("temp dir");
        let enlace = enlaces.path().join("musica");
        std::os::unix::fs::symlink(musica.path(), &enlace).expect("enlazar");
        let por_el_enlace = vec![enlace.to_string_lossy().to_string()];

        let primero = scan_folders_into(&mut conn, &por_el_enlace).expect("barrer");
        assert_eq!(primero.inserted_tracks, 1);

        let segundo = scan_folders_into(&mut conn, &por_el_enlace).expect("barrer");

        assert_eq!(segundo.unchanged_tracks, 1, "tendría que reconocerlo");
        assert_eq!(segundo.inserted_tracks, 0);
    }

    #[test]
    fn la_fecha_guardada_es_la_del_archivo_y_no_la_del_enlace() {
        // El recorrido no sigue enlaces, así que para un enlace la entrada
        // describe el enlace. La fila se guarda con la ruta del archivo
        // apuntado: si le quedara la fecha del enlace, editar el archivo no se
        // notaría nunca.
        //
        // El archivo de verdad vive **fuera** de la carpeta barrida a propósito:
        // con los dos adentro, el recorrido ve también el archivo y lo pisa con
        // su propia fecha, y la prueba pasa aunque el enlace esté mal leído.
        let (_base, musica, mut conn) = biblioteca();
        let afuera = tempfile::tempdir().expect("temp dir");
        let real = archivo_etiquetado(
            afuera.path(),
            "real.mp3",
            &[(ItemKey::TrackTitle, "Un tema")],
        );
        fechar(&real, 1_700_000_000);

        // El enlace se crea ahora, así que su fecha propia es la de hoy: es lo
        // que distingue una de la otra. (No se la puede fijar a mano desde la
        // biblioteca estándar: abrir un enlace para escribir abre el archivo
        // apuntado, que fue lo primero que hizo fallar esta prueba.)
        std::os::unix::fs::symlink(&real, musica.path().join("enlace.mp3")).expect("enlazar");

        barrer(&mut conn, &musica);

        let conocidos =
            known_mtimes_under(&conn, &afuera.path().to_string_lossy()).expect("consultar");
        assert_eq!(
            conocidos.get(&real.to_string_lossy().to_string()),
            Some(&1_700_000_000),
            "la fecha tiene que ser la del archivo apuntado"
        );
    }

    #[test]
    fn una_carpeta_que_falta_no_deja_anotada_la_version() {
        // Si se anotara, el barrido siguiente daría por releída una biblioteca
        // que quedó a medias y ya no volvería a mirarla.
        let (_base, musica, mut conn) = biblioteca();
        archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);

        scan_folders_into(
            &mut conn,
            &[
                musica.path().to_string_lossy().to_string(),
                "/carpeta/que/no/existe".to_string(),
            ],
        )
        .expect("barrer");

        assert_eq!(get_setting(&conn, SCAN_VERSION_SETTING), None);
    }

    #[test]
    fn un_archivo_ilegible_no_deja_anotada_la_version_y_se_reintenta() {
        let (_base, musica, mut conn) = biblioteca();
        let audio = archivo_etiquetado(musica.path(), "a.mp3", &[(ItemKey::TrackTitle, "Un tema")]);
        fechar(&audio, 1_700_000_000);
        barrer(&mut conn, &musica);
        assert_eq!(
            get_setting(&conn, SCAN_VERSION_SETTING).as_deref(),
            Some(SCAN_VERSION)
        );

        // El archivo sigue estando y con fecha nueva, pero ya no se puede leer.
        std::fs::write(&audio, b"esto ya no es un mp3").expect("romper");
        fechar(&audio, 1_700_000_900);

        let resumen = barrer(&mut conn, &musica);
        assert_eq!(resumen.failed_files, 1);

        // La fila queda sin fecha, así que el barrido siguiente lo vuelve a
        // intentar en vez de darlo por bueno sin abrirlo.
        let conocidos =
            known_mtimes_under(&conn, &musica.path().to_string_lossy()).expect("consultar");
        assert_eq!(
            conocidos.get(&audio.to_string_lossy().to_string()),
            Some(&0)
        );

        let tercero = barrer(&mut conn, &musica);
        assert_eq!(tercero.failed_files, 1, "lo reintenta");
        assert_eq!(tercero.unchanged_tracks, 0);
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
