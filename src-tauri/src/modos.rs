//! Cómo está el reproductor: si repite y si suena salteado.
//!
//! La cola vive en la ventana, así que acá no se decide nada de la
//! reproducción. Esto existe para que **MPRIS pueda decir la verdad**: el panel
//! del escritorio lee `LoopStatus` y `Shuffle` de ahí, y hasta ahora los dos
//! devolvían un valor fijo —«None» y «false»— que no tenía nada que ver con lo
//! que estaba pasando. Peor que no tenerlos: los botones se dibujan igual y no
//! hacen nada.
//!
//! Va en los dos sentidos. La ventana avisa cuando cambian, y cuando alguien los
//! toca desde el panel se emite un evento que la ventana atiende — el mismo
//! camino que ya usan las teclas de anterior y siguiente.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Los tres valores de `LoopStatus` que define MPRIS.
pub const SIN_REPETICION: &str = "None";
pub const REPETIR_UNA: &str = "Track";
pub const REPETIR_TODO: &str = "Playlist";

/// Cómo llama la ventana a cada uno.
const NINGUNA: &str = "ninguna";
const UNO: &str = "uno";
const TODO: &str = "todo";

/// Lo que MPRIS tiene que poder contestar.
#[derive(Default)]
pub struct ModosDeReproduccion {
    repeticion: Mutex<String>,
    aleatorio: AtomicBool,
}

impl ModosDeReproduccion {
    pub fn nuevo() -> Self {
        Self {
            repeticion: Mutex::new(NINGUNA.to_string()),
            aleatorio: AtomicBool::new(false),
        }
    }

    /// Guarda lo que dice la ventana. Un nombre desconocido se toma como
    /// «ninguna»: es el valor que no sorprende a nadie.
    pub fn actualizar(&self, repeticion: &str, aleatorio: bool) {
        if let Ok(mut guardada) = self.repeticion.lock() {
            *guardada = match repeticion {
                UNO | TODO => repeticion.to_string(),
                _ => NINGUNA.to_string(),
            };
        }
        self.aleatorio.store(aleatorio, Ordering::Relaxed);
    }

    pub fn aleatorio(&self) -> bool {
        self.aleatorio.load(Ordering::Relaxed)
    }

    /// El `LoopStatus` que espera MPRIS.
    pub fn loop_status(&self) -> String {
        let repeticion = self
            .repeticion
            .lock()
            .map(|guardada| guardada.clone())
            .unwrap_or_else(|_| NINGUNA.to_string());

        nombre_mpris(&repeticion).to_string()
    }
}

/// De cómo lo llama la ventana a cómo lo llama MPRIS.
pub fn nombre_mpris(repeticion: &str) -> &'static str {
    match repeticion {
        UNO => REPETIR_UNA,
        TODO => REPETIR_TODO,
        _ => SIN_REPETICION,
    }
}

/// De cómo lo llama MPRIS a cómo lo llama la ventana.
///
/// `None` para un valor que MPRIS no define: quien lo mandó se equivocó, y
/// cambiar el modo a otra cosa sería peor que no hacer nada.
pub fn nombre_de_la_ventana(loop_status: &str) -> Option<&'static str> {
    match loop_status {
        SIN_REPETICION => Some(NINGUNA),
        REPETIR_UNA => Some(UNO),
        REPETIR_TODO => Some(TODO),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_nombres_van_y_vuelven() {
        for (ventana, mpris) in [
            (NINGUNA, SIN_REPETICION),
            (UNO, REPETIR_UNA),
            (TODO, REPETIR_TODO),
        ] {
            assert_eq!(nombre_mpris(ventana), mpris);
            assert_eq!(nombre_de_la_ventana(mpris), Some(ventana));
        }
    }

    #[test]
    fn un_loop_status_que_mpris_no_define_no_se_acepta() {
        // Cambiar el modo a otra cosa sería peor que no hacer nada.
        for invalido in ["Todo", "playlist", "", "None "] {
            assert_eq!(nombre_de_la_ventana(invalido), None, "{invalido}");
        }
    }

    #[test]
    fn un_nombre_desconocido_de_la_ventana_queda_en_ninguna() {
        let modos = ModosDeReproduccion::nuevo();

        modos.actualizar("lo-que-sea", false);

        assert_eq!(modos.loop_status(), SIN_REPETICION);
    }

    #[test]
    fn lo_que_guarda_la_ventana_es_lo_que_lee_mpris() {
        let modos = ModosDeReproduccion::nuevo();

        modos.actualizar(TODO, true);

        assert_eq!(modos.loop_status(), REPETIR_TODO);
        assert!(modos.aleatorio());

        modos.actualizar(UNO, false);

        assert_eq!(modos.loop_status(), REPETIR_UNA);
        assert!(!modos.aleatorio());
    }
}
