//! Lo que estás escuchando, en tu perfil de Discord.
//!
//! Discord no habla por red sino por un socket local, y la biblioteca que lo
//! maneja es **bloqueante**: conectar, mandar y cerrar esperan. Nada de eso
//! puede pasar ni en el hilo de la interfaz ni cerca del audio, así que el
//! cliente vive en un hilo propio y se le habla por un canal. Quien avisa que
//! cambió la canción deja el mensaje y sigue; si Discord no está abierto, o el
//! socket se cayó, el que espera es ese hilo y nadie más.
//!
//! Que Discord no esté instalado es el caso normal, no un error: sin socket, el
//! reproductor funciona igual y esto no dice nada.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::Deserialize;

/// La imagen por omisión: la que se subió como recurso de la aplicación en el
/// portal de Discord con este nombre. Si el disco no tiene tapa accesible, se
/// muestra el logo de VasakOS.
const IMAGEN_POR_OMISION: &str = "vasakos";

/// El recurso que se superpone en chico cuando está en pausa.
const IMAGEN_EN_PAUSA: &str = "pausa";

/// Cuánto se espera antes de volver a intentar la conexión.
///
/// Sin esto, con Discord cerrado cada cambio de canción intentaría abrir el
/// socket de nuevo. Es un socket local y fallar es barato, pero no gratis.
const ESPERA_ENTRE_INTENTOS: Duration = Duration::from_secs(30);

/// Lo que está sonando, tal como lo manda la interfaz.
#[derive(Debug, Clone, PartialEq)]
pub struct Presencia {
    pub title: String,
    pub artist: String,
    pub album_art_url: Option<String>,
    pub is_paused: bool,
    pub duration_secs: u64,
    pub current_time_secs: u64,
}

enum Mensaje {
    Actualizar(Box<Presencia>),
    Limpiar,
    Cerrar,
}

/// El canal hacia el hilo que habla con Discord.
pub struct DiscordPresence {
    envio: Option<Sender<Mensaje>>,
}

impl DiscordPresence {
    /// Arranca el hilo, o no arranca nada si no hay identificador configurado.
    ///
    /// Sin identificador de aplicación Discord no tiene con qué asociar la
    /// presencia, así que la función queda apagada en silencio: es lo que le
    /// pasa a cualquiera que no haya creado la aplicación en el portal.
    pub fn iniciar() -> Self {
        let Some(app_id) = app_id() else {
            eprintln!(
                "[discord] Sin identificador de aplicación: la presencia queda apagada. \
                 Se configura con VASAK_DISCORD_APP_ID o con `resonance.discord_app_id` \
                 en ~/.config/vasak/vasak.conf"
            );
            return Self { envio: None };
        };

        let (envio, recepcion) = mpsc::channel::<Mensaje>();

        std::thread::Builder::new()
            .name("discord-presence".into())
            .spawn(move || atender(&app_id, &recepcion))
            .map_or_else(
                |error| {
                    eprintln!("[discord] No se pudo crear el hilo de la presencia: {error}");
                    Self { envio: None }
                },
                |_| Self { envio: Some(envio) },
            )
    }

    pub fn actualizar(&self, presencia: Presencia) {
        self.enviar(Mensaje::Actualizar(Box::new(presencia)));
    }

    pub fn limpiar(&self) {
        self.enviar(Mensaje::Limpiar);
    }

    /// Cierra la conexión y deja el perfil como estaba. Se llama al salir.
    pub fn cerrar(&self) {
        self.enviar(Mensaje::Cerrar);
    }

    fn enviar(&self, mensaje: Mensaje) {
        // Si el hilo murió, el canal está cerrado: no hay nada que hacer más
        // que seguir. La música no depende de esto.
        if let Some(envio) = &self.envio {
            let _ = envio.send(mensaje);
        }
    }
}

/// El hilo: mantiene la conexión y aplica lo que llega.
fn atender(app_id: &str, recepcion: &mpsc::Receiver<Mensaje>) {
    let mut cliente: Option<DiscordIpcClient> = None;
    let mut proximo_intento = Instant::now();

    while let Ok(mensaje) = recepcion.recv() {
        match mensaje {
            Mensaje::Cerrar => {
                if let Some(mut abierto) = cliente.take() {
                    let _ = abierto.clear_activity();
                    let _ = abierto.close();
                }
                return;
            }
            Mensaje::Limpiar => {
                if let Some(abierto) = cliente.as_mut() {
                    if abierto.clear_activity().is_err() {
                        cliente = None;
                        proximo_intento = Instant::now() + ESPERA_ENTRE_INTENTOS;
                    }
                }
            }
            Mensaje::Actualizar(presencia) => {
                if cliente.is_none() {
                    if Instant::now() < proximo_intento {
                        continue;
                    }

                    cliente = conectar(app_id);

                    if cliente.is_none() {
                        proximo_intento = Instant::now() + ESPERA_ENTRE_INTENTOS;
                        continue;
                    }
                }

                let Some(abierto) = cliente.as_mut() else {
                    continue;
                };

                if aplicar(abierto, &presencia).is_err() {
                    // Discord se cerró en el medio: se suelta el cliente y se
                    // vuelve a intentar con el próximo cambio de canción.
                    cliente = None;
                    proximo_intento = Instant::now() + ESPERA_ENTRE_INTENTOS;
                }
            }
        }
    }
}

fn conectar(app_id: &str) -> Option<DiscordIpcClient> {
    let mut cliente = DiscordIpcClient::new(app_id);

    match cliente.connect() {
        Ok(()) => Some(cliente),
        Err(error) => {
            eprintln!("[discord] No se pudo conectar: {error}");
            None
        }
    }
}

fn aplicar(
    cliente: &mut DiscordIpcClient,
    presencia: &Presencia,
) -> Result<(), discord_rich_presence::error::Error> {
    let imagen = imagen_grande(presencia);
    let mut assets = Assets::new().large_image(imagen).large_text(&presencia.title);

    if presencia.is_paused {
        assets = assets.small_image(IMAGEN_EN_PAUSA).small_text("En pausa");
    }

    let mut activity = Activity::new()
        .details(&presencia.title)
        .state(&presencia.artist)
        .assets(assets);

    if let Some((inicio, fin)) = tiempos(presencia, ahora()) {
        activity = activity.timestamps(Timestamps::new().start(inicio).end(fin));
    }

    cliente.set_activity(activity)
}

/// Desde cuándo y hasta cuándo, para la barra de progreso de Discord.
///
/// En pausa no hay ninguna: una barra que sigue corriendo con la música
/// detenida es peor que no tener barra. El inicio se calcula hacia atrás desde
/// ahora, que es la única forma de decirle a Discord «va por el minuto dos».
fn tiempos(presencia: &Presencia, ahora: i64) -> Option<(i64, i64)> {
    if presencia.is_paused || presencia.duration_secs == 0 {
        return None;
    }

    let transcurrido = presencia.current_time_secs.min(presencia.duration_secs) as i64;
    let inicio = ahora - transcurrido;

    Some((inicio, inicio + presencia.duration_secs as i64))
}

/// Qué imagen mostrar en grande.
///
/// Discord dibuja la tapa desde **su** lado: sólo sirve una dirección web a la
/// que pueda llegar. Un archivo del disco o una imagen incrustada no se pueden
/// mostrar por más que estén a la vista en el reproductor, así que ahí va el
/// logo del sistema.
fn imagen_grande(presencia: &Presencia) -> &str {
    match presencia.album_art_url.as_deref() {
        Some(url) if url.starts_with("http://") || url.starts_with("https://") => url,
        _ => IMAGEN_POR_OMISION,
    }
}

fn ahora() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|desde| desde.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Deserialize)]
struct ConfigDeVasak {
    resonance: Option<ConfigDeResonance>,
}

#[derive(Deserialize)]
struct ConfigDeResonance {
    discord_app_id: Option<String>,
}

/// El identificador de la aplicación de Discord.
///
/// Primero la variable de entorno, que es lo que sirve para probar sin tocar
/// nada; después la configuración del sistema. No hay ninguno compilado
/// adentro: es de cada instalación, y uno inventado haría que la presencia
/// apareciera con el nombre de otra aplicación.
fn app_id() -> Option<String> {
    if let Ok(valor) = std::env::var("VASAK_DISCORD_APP_ID") {
        let valor = valor.trim().to_string();
        if !valor.is_empty() {
            return Some(valor);
        }
    }

    let ruta = dirs_config()?.join("vasak").join("vasak.conf");
    let contenido = std::fs::read_to_string(ruta).ok()?;
    let config: ConfigDeVasak = serde_json::from_str(&contenido).ok()?;

    config
        .resonance?
        .discord_app_id
        .map(|valor| valor.trim().to_string())
        .filter(|valor| !valor.is_empty())
}

fn dirs_config() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }

    std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sonando() -> Presencia {
        Presencia {
            title: "Bohemian Rhapsody".into(),
            artist: "Queen".into(),
            album_art_url: None,
            is_paused: false,
            duration_secs: 355,
            current_time_secs: 60,
        }
    }

    #[test]
    fn la_barra_empieza_donde_va_la_canción() {
        let (inicio, fin) = tiempos(&sonando(), 1_000).expect("sonando tiene barra");

        assert_eq!(inicio, 940, "un minuto atrás");
        assert_eq!(fin, 940 + 355);
    }

    #[test]
    fn en_pausa_no_hay_barra() {
        let presencia = Presencia {
            is_paused: true,
            ..sonando()
        };

        assert_eq!(tiempos(&presencia, 1_000), None);
    }

    #[test]
    fn sin_duración_tampoco() {
        // Una radio no tiene final: una barra que no llega a ningún lado sólo
        // confunde.
        let presencia = Presencia {
            duration_secs: 0,
            ..sonando()
        };

        assert_eq!(tiempos(&presencia, 1_000), None);
    }

    #[test]
    fn un_tiempo_pasado_del_final_no_corre_la_barra_hacia_atrás() {
        let presencia = Presencia {
            current_time_secs: 900,
            ..sonando()
        };

        let (inicio, fin) = tiempos(&presencia, 1_000).expect("sigue sonando");

        assert_eq!(inicio, 1_000 - 355);
        assert_eq!(fin, 1_000);
    }

    #[test]
    fn la_tapa_se_manda_sólo_si_discord_puede_verla() {
        let con_web = Presencia {
            album_art_url: Some("https://ejemplo/tapa.jpg".into()),
            ..sonando()
        };
        assert_eq!(imagen_grande(&con_web), "https://ejemplo/tapa.jpg");
    }

    #[test]
    fn una_tapa_del_disco_local_cae_en_el_logo() {
        // Discord dibuja la imagen desde su lado: un archivo de esta máquina no
        // lo puede ver, y una imagen incrustada tampoco.
        for url in [
            "/home/alguien/musica/tapa.jpg",
            "file:///home/alguien/musica/tapa.jpg",
            "data:image/png;base64,AAAA",
            "asset://localhost/tapa.jpg",
        ] {
            let presencia = Presencia {
                album_art_url: Some(url.into()),
                ..sonando()
            };

            assert_eq!(imagen_grande(&presencia), IMAGEN_POR_OMISION, "{url}");
        }
    }

    #[test]
    fn sin_tapa_va_el_logo() {
        assert_eq!(imagen_grande(&sonando()), IMAGEN_POR_OMISION);
    }
}
