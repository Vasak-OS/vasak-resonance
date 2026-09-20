//! El registro de lo que se escuchó.
//!
//! Una fila por reproducción, con la ruta y el momento. Nada más: el título, el
//! artista y el álbum ya están en `tracks`, y duplicarlos acá los dejaría
//! congelados el día que alguien corrija una etiqueta.
//!
//! **Cuándo cuenta una reproducción.** Cuando el tema pasó la mitad, o los
//! cuatro minutos, lo que pase primero; y nunca si dura menos de treinta
//! segundos. Es la regla de Last.fm, y el motivo de que exista es que saltear un
//! tema a los diez segundos no es haberlo escuchado. Que además sea la misma
//! regla que usa el scrobbling deja el camino hecho si alguna vez entra.
//!
//! **Lo que no distingue.** El umbral mira la posición, no el tiempo realmente
//! escuchado: saltar al 90% y esperar cuenta como escuchado. Es como se comporta
//! casi cualquier reproductor de escritorio, y perseguirlo pediría llevar la
//! cuenta de cada tramo reproducido para algo que nadie hace sin querer.
//!
//! **La radio no cuenta.** Una emisora no es un tema, y mezclarla ensuciaría
//! cualquier orden por lo más escuchado. No hace falta nombrarla: un stream no
//! tiene duración conocida, y sin duración no hay umbral que pasar.

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::structs::PlaybackProgressEvent;

/// Menos que esto no cuenta nunca, dure lo que dure la escucha.
const MINIMO_SEGUNDOS: u64 = 30;
/// El techo del umbral: cuatro minutos.
///
/// Sin él, un tema de veinte minutos pediría diez para contar, y a esa altura
/// hace rato que se lo está escuchando.
const TOPE_SEGUNDOS: u64 = 240;
/// Volver acá abajo después de haber contado es empezarlo de nuevo.
///
/// Un segundo y no más: la idea es reconocer que el tema arrancó otra vez, no
/// que alguien rebobinó para repetir el estribillo —eso es la misma escucha—.
const REINICIO_SEGUNDOS: u64 = 1;

/// A partir de qué segundo cuenta un tema de esta duración.
///
/// `None` para los que no cuentan nunca: los muy cortos y los que no tienen
/// duración conocida, que es el caso de la radio.
pub fn umbral(duration_seconds: Option<u64>) -> Option<u64> {
    let duracion = duration_seconds?;
    if duracion < MINIMO_SEGUNDOS {
        return None;
    }

    Some((duracion / 2).min(TOPE_SEGUNDOS))
}

/// Lo que se sabe del tema que suena ahora.
struct Cursada {
    path: String,
    /// Si esta vuelta ya quedó anotada. Una escucha es una fila, no una por tic.
    anotada: bool,
}

/// Decide qué anotar, mirando pasar los estados de reproducción.
///
/// Separado de la escritura a propósito: acá está toda la regla y se la puede
/// probar sin base de datos ni hilo de audio.
#[derive(Default)]
pub struct Historial {
    actual: Option<Cursada>,
}

impl Historial {
    /// Devuelve la ruta a anotar, y sólo la primera vez que el tema pasa el
    /// umbral. En cualquier otro tic devuelve `None`.
    pub fn observar(
        &mut self,
        path: Option<&str>,
        position_seconds: u64,
        duration_seconds: Option<u64>,
    ) -> Option<String> {
        let Some(path) = path else {
            // No suena nada: la próxima vez que suene algo es otra escucha.
            self.actual = None;
            return None;
        };

        let empieza_de_nuevo = match &self.actual {
            // Otro tema, o el mismo ya anotado y de vuelta en el principio: lo
            // pusieron otra vez, y eso es otra escucha.
            Some(cursada) => {
                cursada.path != path || (cursada.anotada && position_seconds <= REINICIO_SEGUNDOS)
            }
            None => true,
        };

        if empieza_de_nuevo {
            self.actual = Some(Cursada {
                path: path.to_string(),
                anotada: false,
            });
        }

        let umbral = umbral(duration_seconds)?;
        let cursada = self.actual.as_mut()?;

        if cursada.anotada || position_seconds < umbral {
            return None;
        }

        cursada.anotada = true;
        Some(cursada.path.clone())
    }
}

static HISTORIAL: OnceLock<Mutex<Historial>> = OnceLock::new();

/// Mira un estado de reproducción y anota si corresponde.
///
/// Se la llama desde el hilo de audio, dos veces por segundo. Casi siempre no
/// hace nada: la decisión es un par de comparaciones, y la base sólo se toca una
/// vez por tema.
///
/// Que falle no se le cuenta a nadie. Esto es contabilidad: la música no puede
/// cortarse porque la base esté ocupada.
pub fn anotar_si_corresponde(snapshot: &PlaybackProgressEvent) {
    let historial = HISTORIAL.get_or_init(|| Mutex::new(Historial::default()));

    let Ok(mut historial) = historial.lock() else {
        return;
    };

    let Some(path) = historial.observar(
        snapshot.path.as_deref(),
        snapshot.position_seconds,
        snapshot.duration_seconds,
    ) else {
        return;
    };

    // Fuera del candado del historial no hace falta: la escritura es una fila en
    // una base local y el hilo de audio es uno solo.
    if let Err(error) = anotar(&path) {
        eprintln!("[historial] no se pudo anotar la reproducción: {error}");
    }
}

fn anotar(path: &str) -> Result<(), String> {
    let momento = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs() as i64;

    let db_path = crate::db::get_database_path()?;
    let conn = crate::db::open_database(&db_path)?;

    crate::db::record_play(&conn, path, momento)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un tema de tres minutos cuenta a la mitad: al minuto y medio.
    const DURACION: Option<u64> = Some(180);

    #[test]
    fn el_umbral_es_la_mitad_del_tema() {
        assert_eq!(umbral(Some(180)), Some(90));
        assert_eq!(umbral(Some(60)), Some(30));
    }

    #[test]
    fn pero_nunca_mas_de_cuatro_minutos() {
        // Un tema de veinte minutos no pide diez: a los cuatro ya se lo está
        // escuchando.
        assert_eq!(umbral(Some(1200)), Some(240));
    }

    #[test]
    fn un_tema_muy_corto_no_cuenta_nunca() {
        assert_eq!(umbral(Some(29)), None);
        assert_eq!(umbral(Some(5)), None);
    }

    #[test]
    fn sin_duracion_conocida_no_hay_umbral() {
        // Es el caso de la radio, y es la única razón por la que no hace falta
        // nombrarla en ningún lado.
        assert_eq!(umbral(None), None);
    }

    #[test]
    fn se_anota_al_pasar_la_mitad_y_una_sola_vez() {
        let mut historial = Historial::default();

        assert_eq!(historial.observar(Some("/m/a.mp3"), 0, DURACION), None);
        assert_eq!(historial.observar(Some("/m/a.mp3"), 89, DURACION), None);
        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
        // El resto del tema son cien tics más que no tienen que anotar nada.
        assert_eq!(historial.observar(Some("/m/a.mp3"), 91, DURACION), None);
        assert_eq!(historial.observar(Some("/m/a.mp3"), 179, DURACION), None);
    }

    #[test]
    fn saltear_un_tema_antes_de_la_mitad_no_lo_cuenta() {
        // El motivo por el que la regla existe.
        let mut historial = Historial::default();

        assert_eq!(historial.observar(Some("/m/a.mp3"), 10, DURACION), None);
        assert_eq!(historial.observar(Some("/m/b.mp3"), 0, DURACION), None);
        assert_eq!(historial.observar(Some("/m/b.mp3"), 12, DURACION), None);
    }

    #[test]
    fn cada_tema_cuenta_por_su_cuenta() {
        let mut historial = Historial::default();

        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
        assert_eq!(
            historial.observar(Some("/m/b.mp3"), 90, DURACION),
            Some("/m/b.mp3".to_string())
        );
    }

    #[test]
    fn ponerlo_de_nuevo_es_otra_escucha() {
        let mut historial = Historial::default();

        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
        // Vuelve al principio: lo pusieron otra vez.
        assert_eq!(historial.observar(Some("/m/a.mp3"), 0, DURACION), None);
        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
    }

    #[test]
    fn rebobinar_para_repetir_el_estribillo_no_es_otra_escucha() {
        // La diferencia con la prueba de arriba es dónde cae: volver al segundo
        // cuarenta es seguir escuchando el mismo tema.
        let mut historial = Historial::default();

        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
        assert_eq!(historial.observar(Some("/m/a.mp3"), 40, DURACION), None);
        assert_eq!(historial.observar(Some("/m/a.mp3"), 95, DURACION), None);
    }

    #[test]
    fn una_emisora_de_radio_no_se_anota_por_larga_que_sea() {
        let mut historial = Historial::default();

        for segundo in [0, 90, 240, 3600] {
            assert_eq!(
                historial.observar(Some("http://emisora/stream"), segundo, None),
                None
            );
        }
    }

    #[test]
    fn el_silencio_cierra_la_escucha() {
        let mut historial = Historial::default();

        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
        // Se paró la música y se volvió a poner el mismo tema: otra escucha.
        assert_eq!(historial.observar(None, 0, None), None);
        assert_eq!(
            historial.observar(Some("/m/a.mp3"), 90, DURACION),
            Some("/m/a.mp3".to_string())
        );
    }
}
