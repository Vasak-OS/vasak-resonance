//! Lo que se le puede pedir al reproductor desde afuera de la ventana.
//!
//! Hay tres entradas que piden lo mismo: el panel por MPRIS, el menú del icono
//! de la bandeja, y las teclas multimedia. Cuando cada una tiene su copia
//! terminan portándose distinto en algún caso —el que nadie prueba— y no se
//! nota hasta que pasa. Acá está una sola vez, y las tres llaman a esto.

use tauri::{AppHandle, Emitter, Manager};

use crate::audio_manager::AudioState;

/// El evento que la ventana escucha para pasar al tema siguiente.
///
/// El nombre arranca con `mpris-` porque MPRIS fue el primero en emitirlo, y
/// cambiarlo ahora rompería a quien lo escucha del lado de la ventana sin que
/// nada avise.
pub const EVENTO_SIGUIENTE: &str = "mpris-next-request";

/// El evento que la ventana escucha para volver al tema anterior.
pub const EVENTO_ANTERIOR: &str = "mpris-previous-request";

/// Órdenes que llegan de afuera de la ventana.
///
/// Es un rasgo y no una estructura suelta para poder probar quién llama a qué
/// sin levantar una aplicación de Tauri entera.
pub trait Mando: Send + Sync + 'static {
    /// Alterna entre sonar y estar en pausa.
    fn reproducir_o_pausar(&self) -> Result<(), String>;

    /// Pasa al tema siguiente de la cola.
    fn siguiente(&self) -> Result<(), String>;

    /// Vuelve al tema anterior.
    fn anterior(&self) -> Result<(), String>;

    /// Trae la ventana principal al frente.
    fn mostrar(&self) -> Result<(), String>;

    /// Cierra la aplicación.
    fn salir(&self);

    /// Si hay algo sonando ahora mismo.
    fn esta_sonando(&self) -> bool;
}

/// El mando de verdad: el que habla con el hilo de audio y con la ventana.
#[derive(Clone)]
pub struct MandoDeTauri {
    app_handle: AppHandle,
    audio_state: AudioState,
}

impl MandoDeTauri {
    pub fn nuevo(app_handle: AppHandle, audio_state: AudioState) -> Self {
        Self {
            app_handle,
            audio_state,
        }
    }
}

impl Mando for MandoDeTauri {
    fn reproducir_o_pausar(&self) -> Result<(), String> {
        self.audio_state.play_pause_toggle()
    }

    fn siguiente(&self) -> Result<(), String> {
        self.app_handle
            .emit(EVENTO_SIGUIENTE, ())
            .map_err(|error| format!("No se pudo emitir evento next: {error}"))
    }

    fn anterior(&self) -> Result<(), String> {
        self.app_handle
            .emit(EVENTO_ANTERIOR, ())
            .map_err(|error| format!("No se pudo emitir evento previous: {error}"))
    }

    /// Mostrar, des-minimizar, enfocar, y pedirle al compositor que la levante.
    ///
    /// Los tres primeros hacen falta y **no alcanzan**: en Wayland un cliente
    /// no se puede enfocar solo, así que `set_focus` contesta `Ok` y la ventana
    /// se queda atrás. Es por qué tocar el tema en el panel —`Raise` de
    /// MPRIS— parecía no hacer nada: la vista quedaba con `activated: false`.
    /// El cuarto paso se lo pide a Wayfire, y fuera de Wayfire no hace nada.
    fn mostrar(&self) -> Result<(), String> {
        let Some(ventana) = self.app_handle.get_webview_window("main") else {
            return Err("No se encontró la ventana principal".to_string());
        };

        ventana.show().map_err(|error| error.to_string())?;
        ventana.unminimize().map_err(|error| error.to_string())?;
        ventana.set_focus().map_err(|error| error.to_string())?;

        // Aparte y sin esperarlo: esto lo llaman un manejador de D-Bus y el
        // menú de la bandeja, y ninguno de los dos puede quedarse esperando a
        // que conteste el compositor. El título se lo preguntamos a la ventana
        // en vez de suponerlo, que es lo único que Wayfire tiene para
        // distinguirla del mini-reproductor.
        #[cfg(target_os = "linux")]
        if let Ok(titulo) = ventana.title() {
            tauri::async_runtime::spawn(async move {
                if let Err(error) = crate::wayfire_ipc::enfocar_ventana_propia(titulo).await {
                    eprintln!("No se pudo traer la ventana al frente: {error}");
                }
            });
        }

        Ok(())
    }

    fn salir(&self) {
        self.app_handle.exit(0);
    }

    fn esta_sonando(&self) -> bool {
        self.audio_state
            .playback_snapshot()
            .map(|estado| estado.is_playing)
            .unwrap_or(false)
    }
}
