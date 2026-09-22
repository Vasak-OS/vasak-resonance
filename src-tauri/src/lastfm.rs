//! Lo que estás escuchando, en tu perfil de Last.fm.
//!
//! **Apagado salvo que alguien lo encienda.** Hacen falta dos cosas: una clave
//! de API propia —como el identificador de Discord, por lo mismo: este
//! repositorio es público y una clave compilada adentro la tiene cualquiera— y
//! una vuelta de autorización en el navegador. Sin las dos, esto no existe: no
//! habla, no avisa y no se ve.
//!
//! **Cuándo cuenta una escucha** no se decide acá. Lo decide
//! [`crate::historial`], que ya lleva esa cuenta para la biblioteca local con
//! **la misma regla que usa Last.fm** —la mitad del tema o cuatro minutos, lo
//! que pase primero, nunca por debajo de treinta segundos—. Que fuera la misma
//! regla era el motivo de escribirla así.
//!
//! **Que falle no se le cuenta a nadie.** Un scrobble que no sale es una línea
//! en el diario. La red se cae, Last.fm se cae, y nada de eso puede cortar la
//! música ni aparecer en la cara de quien está escuchando.
//!
//! La red es bloqueante y el hilo de audio llama a esto dos veces por segundo,
//! así que —igual que con Discord— el que habla es un hilo propio y se le deja
//! el mensaje en un canal.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use md5::{Digest, Md5};
use serde::Deserialize;

use crate::audio::{ALBUM_DESCONOCIDO, ARTISTA_DESCONOCIDO, TITULO_DESCONOCIDO};

/// Dónde se le habla a la API.
const RAIZ: &str = "https://ws.audioscrobbler.com/2.0/";

/// La página donde la persona autoriza la aplicación.
pub const PAGINA_DE_AUTORIZACION: &str = "https://www.last.fm/api/auth/";

/// Cuánto se espera a la red antes de darlo por perdido.
///
/// Hay un hilo solo atendiendo la cola: sin tope, una conexión colgada deja sin
/// mandar todo lo que venga atrás.
const ESPERA_DE_RED: Duration = Duration::from_secs(10);

/// Dónde queda la sesión, en la tabla `settings` de la base.
const CLAVE_DE_SESION: &str = "lastfm_session_key";
const CLAVE_DE_USUARIO: &str = "lastfm_username";

/// Una escucha, lista para mandar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escucha {
    pub artista: String,
    pub titulo: String,
    /// Sin álbum si las etiquetas no lo dicen: mejor un hueco que «Unknown
    /// Album» escrito en el perfil para siempre.
    pub album: Option<String>,
    pub duracion_segundos: u64,
    /// Cuándo empezó a sonar, en segundos desde la época, que es como Last.fm
    /// lo pide.
    pub momento: i64,
}

/// La sesión que devuelve la autorización. No vence, así que se guarda y listo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sesion {
    pub clave: String,
    pub usuario: String,
}

/// Lo que hay que saber para dibujar —o no dibujar— la sección de ajustes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Estado {
    /// Si hay clave de API. Sin esto no se muestra nada.
    pub configurado: bool,
    /// Con quién está vinculado, si lo está.
    pub usuario: Option<String>,
}

/// Qué escucha se puede mandar, y cuál no.
///
/// Devuelve `None` para lo que no tiene sentido publicar. Un perfil lleno de
/// «Unknown Artist» es peor que un hueco: no dice nada de lo que alguien
/// escuchó y ensucia sus estadísticas para siempre, porque un scrobble no se
/// borra solo.
pub fn escucha_publicable(
    artista: &str,
    titulo: &str,
    album: &str,
    duracion_segundos: u64,
    momento: i64,
) -> Option<Escucha> {
    let artista = utilizable(artista, ARTISTA_DESCONOCIDO)?;
    let titulo = utilizable(titulo, TITULO_DESCONOCIDO)?;

    Some(Escucha {
        artista,
        titulo,
        album: utilizable(album, ALBUM_DESCONOCIDO),
        duracion_segundos,
        momento,
    })
}

/// Un campo que sirve para mandar: ni vacío, ni el relleno de «no se sabe».
fn utilizable(valor: &str, relleno: &str) -> Option<String> {
    let valor = valor.trim();
    if valor.is_empty() || valor.eq_ignore_ascii_case(relleno) {
        return None;
    }

    Some(valor.to_string())
}

/// La firma que Last.fm pide en cada llamada.
///
/// Es el md5 de todos los parámetros —clave y valor pegados, **ordenados por
/// clave**— con el secreto al final. No es una decisión de seguridad nuestra:
/// es lo que la API pide, y de ahí sale el único md5 de este programa.
///
/// `format` queda afuera de la firma, que es por qué no entra nunca a este
/// mapa: se agrega recién al armar el pedido.
pub fn firmar(parametros: &BTreeMap<&str, String>, secreto: &str) -> String {
    let mut crudo = String::new();
    for (clave, valor) in parametros {
        crudo.push_str(clave);
        crudo.push_str(valor);
    }
    crudo.push_str(secreto);

    a_hexadecimal(&Md5::digest(crudo.as_bytes()))
}

/// Los bytes en minúscula, que es como Last.fm espera la firma.
pub fn a_hexadecimal(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut texto, byte| {
        use std::fmt::Write;
        let _ = write!(texto, "{byte:02x}");
        texto
    })
}

/// Las claves de la API, que son de cada instalación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claves {
    pub api_key: String,
    pub secreto: String,
}

#[derive(Deserialize)]
struct ConfigDeVasak {
    resonance: Option<ConfigDeResonance>,
}

#[derive(Deserialize)]
struct ConfigDeResonance {
    lastfm_api_key: Option<String>,
    lastfm_api_secret: Option<String>,
}

/// Las claves de la instalación, o nada.
///
/// Primero el entorno, que es lo que sirve para probar sin tocar la
/// configuración; después `~/.config/vasak/vasak.conf`. **No hay ninguna
/// compilada adentro**: este repositorio es público, así que una clave metida
/// en el binario la saca cualquiera del paquete y queda a nombre nuestro lo que
/// haga con ella. Es la misma decisión que con el identificador de Discord.
fn claves() -> Option<Claves> {
    let del_entorno = |nombre: &str| {
        std::env::var(nombre)
            .ok()
            .map(|valor| valor.trim().to_string())
            .filter(|valor| !valor.is_empty())
    };

    if let (Some(api_key), Some(secreto)) = (
        del_entorno("VASAK_LASTFM_API_KEY"),
        del_entorno("VASAK_LASTFM_API_SECRET"),
    ) {
        return Some(Claves { api_key, secreto });
    }

    let ruta = dirs_config()?.join("vasak").join("vasak.conf");
    let contenido = std::fs::read_to_string(ruta).ok()?;
    let config: ConfigDeVasak = serde_json::from_str(&contenido).ok()?;
    let resonance = config.resonance?;

    claves_de(
        resonance.lastfm_api_key.as_deref(),
        resonance.lastfm_api_secret.as_deref(),
    )
}

/// La misma decisión sin leer nada, para poder probarla.
///
/// Hacen falta **las dos**: con clave y sin secreto no se puede firmar ninguna
/// llamada, así que media configuración es lo mismo que ninguna, y decirlo acá
/// evita que el error aparezca recién al intentar vincular.
fn claves_de(api_key: Option<&str>, secreto: Option<&str>) -> Option<Claves> {
    let limpiar = |valor: Option<&str>| {
        valor
            .map(|valor| valor.trim().to_string())
            .filter(|valor| !valor.is_empty())
    };

    Some(Claves {
        api_key: limpiar(api_key)?,
        secreto: limpiar(secreto)?,
    })
}

fn dirs_config() -> Option<PathBuf> {
    dirs::config_dir().filter(|base| base.is_absolute())
}

/// La sesión guardada, si alguna vez se autorizó.
///
/// Vacío es no vinculado: desvincular deja las filas puestas con la cadena
/// vacía en vez de borrarlas, y una sesión vacía no sirve para firmar nada.
fn sesion_guardada() -> Option<Sesion> {
    let db_path = crate::db::get_database_path().ok()?;
    let conn = crate::db::open_database(&db_path).ok()?;

    let leer = |clave: &str| {
        crate::db::get_setting(&conn, clave)
            .map(|valor| valor.trim().to_string())
            .filter(|valor| !valor.is_empty())
    };

    Some(Sesion {
        clave: leer(CLAVE_DE_SESION)?,
        usuario: leer(CLAVE_DE_USUARIO)?,
    })
}

fn guardar_sesion(sesion: Option<&Sesion>) -> Result<(), String> {
    let db_path = crate::db::get_database_path()?;
    let conn = crate::db::open_database(&db_path)?;

    let (clave, usuario) = match sesion {
        Some(sesion) => (sesion.clave.as_str(), sesion.usuario.as_str()),
        // Desvincular es dejarlas vacías y no borrar la fila: `get_setting` no
        // distingue una de otra, y una cadena vacía no vincula nada.
        None => ("", ""),
    };

    crate::db::set_setting(&conn, CLAVE_DE_SESION, clave)?;
    crate::db::set_setting(&conn, CLAVE_DE_USUARIO, usuario)
}

/// El que habla con la API.
struct Cliente {
    claves: Claves,
    http: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct ErrorDeLastfm {
    error: i64,
    message: String,
}

impl Cliente {
    fn nuevo(claves: Claves) -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .timeout(ESPERA_DE_RED)
            .build()
            .map_err(|error| format!("No se pudo armar el cliente HTTP: {error}"))?;

        Ok(Self { claves, http })
    }

    /// Una llamada firmada.
    ///
    /// `metodo` y `api_key` los pone esto, así que quien llama sólo trae lo
    /// suyo. Los de escritura van por POST porque es lo que la API pide; los de
    /// lectura, por GET.
    fn llamar(
        &self,
        metodo: &str,
        mut parametros: BTreeMap<&str, String>,
        escribe: bool,
    ) -> Result<serde_json::Value, String> {
        parametros.insert("method", metodo.to_string());
        parametros.insert("api_key", self.claves.api_key.clone());

        let firma = firmar(&parametros, &self.claves.secreto);
        parametros.insert("api_sig", firma);
        // Después de firmar: `format` no entra en la firma.
        parametros.insert("format", "json".to_string());

        let pedido = if escribe {
            self.http.post(RAIZ).form(&parametros)
        } else {
            self.http.get(RAIZ).query(&parametros)
        };

        let respuesta = pedido
            .send()
            .map_err(|error| format!("no se pudo llegar a Last.fm: {error}"))?;

        let cuerpo: serde_json::Value = respuesta
            .json()
            .map_err(|error| format!("Last.fm contestó algo que no se entiende: {error}"))?;

        // Un error de la API viene con código 200 y el problema adentro del
        // cuerpo, así que mirar el estado HTTP no alcanza.
        if let Ok(fallo) = serde_json::from_value::<ErrorDeLastfm>(cuerpo.clone()) {
            return Err(format!(
                "Last.fm rechazó la llamada ({}): {}",
                fallo.error, fallo.message
            ));
        }

        Ok(cuerpo)
    }

    /// Pide un token para que alguien lo autorice en el navegador.
    fn pedir_token(&self) -> Result<String, String> {
        let cuerpo = self.llamar("auth.getToken", BTreeMap::new(), false)?;

        cuerpo
            .get("token")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "Last.fm no devolvió ningún token".to_string())
    }

    /// Cambia un token ya autorizado por la sesión, que no vence.
    fn pedir_sesion(&self, token: &str) -> Result<Sesion, String> {
        let mut parametros = BTreeMap::new();
        parametros.insert("token", token.to_string());

        let cuerpo = self.llamar("auth.getSession", parametros, false)?;
        let sesion = cuerpo
            .get("session")
            .ok_or_else(|| "Last.fm no devolvió ninguna sesión".to_string())?;

        let leer = |campo: &str| {
            sesion
                .get(campo)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("la sesión de Last.fm no trae «{campo}»"))
        };

        Ok(Sesion {
            clave: leer("key")?,
            usuario: leer("name")?,
        })
    }

    fn ahora_suena(&self, escucha: &Escucha, sesion: &Sesion) -> Result<(), String> {
        self.llamar(
            "track.updateNowPlaying",
            parametros_de(escucha, sesion, false),
            true,
        )
        .map(|_| ())
    }

    fn scrobble(&self, escucha: &Escucha, sesion: &Sesion) -> Result<(), String> {
        self.llamar("track.scrobble", parametros_de(escucha, sesion, true), true)
            .map(|_| ())
    }
}

/// Los parámetros de una escucha.
///
/// `timestamp` sólo va en el scrobble: en «esto está sonando» no existe, y
/// mandarlo igual haría que la firma no cierre.
fn parametros_de(
    escucha: &Escucha,
    sesion: &Sesion,
    con_momento: bool,
) -> BTreeMap<&'static str, String> {
    let mut parametros = BTreeMap::new();
    parametros.insert("artist", escucha.artista.clone());
    parametros.insert("track", escucha.titulo.clone());
    parametros.insert("sk", sesion.clave.clone());

    if let Some(album) = &escucha.album {
        parametros.insert("album", album.clone());
    }

    if escucha.duracion_segundos > 0 {
        parametros.insert("duration", escucha.duracion_segundos.to_string());
    }

    if con_momento {
        parametros.insert("timestamp", escucha.momento.to_string());
    }

    parametros
}

/// Cuándo empezó a sonar lo que va por el segundo `posicion`.
///
/// Last.fm quiere el momento en que **arrancó** el tema, no aquél en que se
/// alcanzó el umbral. Mandar el segundo deja el historial ordenado por cuándo
/// se terminó de escuchar cada cosa, que no es lo mismo cuando alguien pone
/// algo largo.
pub fn momento_de_inicio(ahora: i64, posicion_segundos: u64) -> i64 {
    // `.max(0)` y no sólo `saturating_sub`, que satura en `i64::MIN` y no en
    // cero: con el reloj del sistema mal puesto saldría un momento negativo, y
    // Last.fm rechaza el scrobble entero.
    ahora.saturating_sub(posicion_segundos as i64).max(0)
}

enum Mensaje {
    Suena(Box<Escucha>),
    Scrobble(Box<Escucha>),
    Sesion(Option<Sesion>),
}

/// El canal hacia el hilo que habla con Last.fm, más las claves.
struct Enlace {
    envio: Mutex<Sender<Mensaje>>,
    claves: Claves,
}

static ENLACE: OnceLock<Option<Enlace>> = OnceLock::new();

/// El enlace, armándolo la primera vez. `None` es «no hay claves», que es el
/// caso de casi todo el mundo y no es un error.
fn enlace() -> Option<&'static Enlace> {
    ENLACE.get_or_init(arrancar).as_ref()
}

fn arrancar() -> Option<Enlace> {
    let claves = claves()?;
    let (envio, recepcion) = mpsc::channel::<Mensaje>();
    let del_hilo = claves.clone();

    std::thread::Builder::new()
        .name("lastfm".into())
        .spawn(move || atender(del_hilo, &recepcion))
        .map_err(|error| eprintln!("[lastfm] No se pudo crear el hilo: {error}"))
        .ok()?;

    Some(Enlace {
        envio: Mutex::new(envio),
        claves,
    })
}

/// El hilo. Espera mensajes y los manda; nada de esto vuelve a quien llamó.
fn atender(claves: Claves, recepcion: &Receiver<Mensaje>) {
    let cliente = match Cliente::nuevo(claves) {
        Ok(cliente) => cliente,
        Err(error) => {
            eprintln!("[lastfm] {error}");
            return;
        }
    };

    let mut sesion = sesion_guardada();

    while let Ok(mensaje) = recepcion.recv() {
        match mensaje {
            // Vincular y desvincular llegan por acá: la sesión se establece con
            // la aplicación abierta, así que leerla sólo al arrancar dejaría el
            // scrobbling apagado hasta el próximo inicio.
            Mensaje::Sesion(nueva) => sesion = nueva,
            Mensaje::Suena(escucha) => {
                if let Some(sesion) = &sesion {
                    if let Err(error) = cliente.ahora_suena(&escucha, sesion) {
                        eprintln!("[lastfm] no se pudo avisar qué está sonando: {error}");
                    }
                }
            }
            Mensaje::Scrobble(escucha) => {
                if let Some(sesion) = &sesion {
                    if let Err(error) = cliente.scrobble(&escucha, sesion) {
                        eprintln!("[lastfm] no se pudo mandar la escucha: {error}");
                    }
                }
            }
        }
    }
}

fn enviar(mensaje: Mensaje) {
    let Some(enlace) = enlace() else {
        return;
    };

    let Ok(envio) = enlace.envio.lock() else {
        return;
    };

    // Que el hilo se haya ido no se le cuenta a nadie: esto es contabilidad.
    let _ = envio.send(mensaje);
}

/// Lo que hay que contarle a Last.fm de este tic de reproducción.
///
/// Casi siempre nada: el hilo de audio llama a esto dos veces por segundo y las
/// dos novedades pasan una vez por tema.
pub fn contar(
    novedades: &crate::historial::Novedades,
    snapshot: &crate::structs::PlaybackProgressEvent,
) {
    if !novedades.empezo && !novedades.anotada {
        return;
    }

    if enlace().is_none() {
        return;
    }

    let Some(sonando) = snapshot.now_playing.as_ref() else {
        return;
    };

    let ahora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|desde| desde.as_secs() as i64)
        .unwrap_or_default();

    let Some(escucha) = escucha_publicable(
        &sonando.artist,
        &sonando.title,
        &sonando.album,
        sonando.duration_seconds,
        momento_de_inicio(ahora, snapshot.position_seconds),
    ) else {
        return;
    };

    if novedades.empezo {
        enviar(Mensaje::Suena(Box::new(escucha.clone())));
    }

    if novedades.anotada {
        enviar(Mensaje::Scrobble(Box::new(escucha)));
    }
}

/// Si hay clave de API y con quién está vinculado.
pub fn estado() -> Estado {
    Estado {
        configurado: enlace().is_some(),
        usuario: sesion_guardada().map(|sesion| sesion.usuario),
    }
}

/// Pide un token y devuelve la dirección que hay que abrir para autorizarlo.
///
/// El token todavía no sirve para nada: recién después de que alguien apruebe
/// en esa página se lo puede cambiar por una sesión, con [`confirmar`].
pub fn pedir_autorizacion() -> Result<(String, String), String> {
    let cliente = cliente_suelto()?;
    let token = cliente.pedir_token()?;
    let direccion = format!(
        "{PAGINA_DE_AUTORIZACION}?api_key={}&token={token}",
        cliente.claves.api_key
    );

    Ok((token, direccion))
}

/// Cambia el token ya aprobado por la sesión, y la guarda.
pub fn confirmar(token: &str) -> Result<Estado, String> {
    let sesion = cliente_suelto()?.pedir_sesion(token)?;
    guardar_sesion(Some(&sesion))?;
    enviar(Mensaje::Sesion(Some(sesion)));

    Ok(estado())
}

/// Desvincula la cuenta. No le avisa a Last.fm: la sesión no vence y lo único
/// que hay que hacer es dejar de usarla.
pub fn desvincular() -> Result<Estado, String> {
    guardar_sesion(None)?;
    enviar(Mensaje::Sesion(None));

    Ok(estado())
}

/// Un cliente para las dos llamadas de autorización, que pasan una vez en la
/// vida y no van por el hilo.
fn cliente_suelto() -> Result<Cliente, String> {
    let enlace = enlace().ok_or_else(|| {
        "Sin clave de API de Last.fm. Se configura con VASAK_LASTFM_API_KEY y \
         VASAK_LASTFM_API_SECRET, o con `resonance.lastfm_api_key` y \
         `resonance.lastfm_api_secret` en ~/.config/vasak/vasak.conf"
            .to_string()
    })?;

    Cliente::nuevo(enlace.claves.clone())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn sesion() -> Sesion {
        Sesion {
            clave: "sk123".to_string(),
            usuario: "alguien".to_string(),
        }
    }

    fn escucha() -> Escucha {
        Escucha {
            artista: "Patricio Rey".to_string(),
            titulo: "La bestia pop".to_string(),
            album: Some("¡Gulp!".to_string()),
            duracion_segundos: 231,
            momento: 1_700_000_000,
        }
    }

    /// El byte 0 tiene que salir «00» y no «0»: una firma a la que le falta un
    /// carácter la rechaza Last.fm entera, y pasa una vez de cada dieciséis.
    #[test]
    fn el_hexadecimal_rellena_con_cero() {
        assert_eq!(a_hexadecimal(&[0, 1, 255, 16]), "0001ff10");
    }

    /// La firma es el md5 de clave y valor pegados, **en orden de clave**, con
    /// el secreto al final. El valor esperado está calculado afuera: si lo
    /// calculara acá con el mismo código, la prueba diría que sí a cualquier
    /// cosa.
    #[test]
    fn la_firma_es_el_md5_de_los_parametros_ordenados() {
        let mut parametros = BTreeMap::new();
        // A propósito al revés: lo que ordena es el mapa, no quien llama.
        parametros.insert("b", "2".to_string());
        parametros.insert("a", "1".to_string());

        assert_eq!(firmar(&parametros, "s"), "1d0396bcbc2c54e569e7af9cf9c4685e");
    }

    #[test]
    fn la_firma_incluye_el_metodo_y_la_clave() {
        let mut parametros = BTreeMap::new();
        parametros.insert("method", "auth.getToken".to_string());
        parametros.insert("api_key", "ABC".to_string());

        assert_eq!(
            firmar(&parametros, "secreto"),
            "496837e25623ea164514748940a79235"
        );
    }

    /// Un perfil lleno de «Unknown Artist» es peor que un hueco, y un scrobble
    /// no se borra solo.
    #[test]
    fn no_se_manda_lo_que_no_tiene_artista() {
        assert_eq!(
            escucha_publicable("Unknown Artist", "Un tema", "Un disco", 200, 1),
            None
        );
        assert_eq!(
            escucha_publicable("   ", "Un tema", "Un disco", 200, 1),
            None
        );
    }

    #[test]
    fn no_se_manda_lo_que_no_tiene_titulo() {
        assert_eq!(
            escucha_publicable("Alguien", "Unknown Title", "Un disco", 200, 1),
            None
        );
        assert_eq!(escucha_publicable("Alguien", "", "Un disco", 200, 1), None);
    }

    /// El disco desconocido **no** descarta la escucha: se manda sin álbum, que
    /// es un hueco que Last.fm sabe llenar solo.
    #[test]
    fn el_disco_desconocido_se_manda_vacio_y_no_descarta() {
        let publicable = escucha_publicable("Alguien", "Un tema", "Unknown Album", 200, 1)
            .expect("la escucha se manda igual");

        assert_eq!(publicable.album, None);
        assert_eq!(publicable.artista, "Alguien");
    }

    #[test]
    fn los_espacios_de_los_costados_no_viajan() {
        let publicable = escucha_publicable("  Alguien  ", " Un tema ", " Un disco ", 200, 1)
            .expect("la escucha se manda");

        assert_eq!(publicable.artista, "Alguien");
        assert_eq!(publicable.titulo, "Un tema");
        assert_eq!(publicable.album.as_deref(), Some("Un disco"));
    }

    /// Media configuración es lo mismo que ninguna: sin secreto no se puede
    /// firmar ni la primera llamada.
    #[test]
    fn hacen_falta_las_dos_claves() {
        assert_eq!(claves_de(Some("abc"), None), None);
        assert_eq!(claves_de(None, Some("xyz")), None);
        assert_eq!(claves_de(Some("abc"), Some("  ")), None);
        assert_eq!(
            claves_de(Some(" abc "), Some(" xyz ")),
            Some(Claves {
                api_key: "abc".to_string(),
                secreto: "xyz".to_string(),
            })
        );
    }

    /// Last.fm quiere cuándo **arrancó** el tema. Mandar el momento en que se
    /// cruzó el umbral deja el historial ordenado por otra cosa.
    #[test]
    fn el_momento_es_el_del_principio_del_tema() {
        assert_eq!(momento_de_inicio(1_000, 120), 880);
        assert_eq!(momento_de_inicio(1_000, 0), 1_000);
    }

    /// Un reloj que todavía no arrancó no puede dar un momento negativo, que
    /// Last.fm rechaza.
    #[test]
    fn el_momento_no_se_va_abajo_de_cero() {
        assert_eq!(momento_de_inicio(10, 120), 0);
    }

    /// `timestamp` es de la escucha terminada. En «esto está sonando» no
    /// existe, y mandarlo igual haría que la firma no cierre del lado de allá.
    #[test]
    fn el_momento_solo_va_en_el_scrobble() {
        let del_scrobble = parametros_de(&escucha(), &sesion(), true);
        let del_ahora_suena = parametros_de(&escucha(), &sesion(), false);

        assert_eq!(
            del_scrobble.get("timestamp").map(String::as_str),
            Some("1700000000")
        );
        assert_eq!(del_ahora_suena.get("timestamp"), None);
    }

    #[test]
    fn los_parametros_llevan_lo_que_hay_que_llevar() {
        let parametros = parametros_de(&escucha(), &sesion(), true);

        assert_eq!(
            parametros.get("artist").map(String::as_str),
            Some("Patricio Rey")
        );
        assert_eq!(
            parametros.get("track").map(String::as_str),
            Some("La bestia pop")
        );
        assert_eq!(parametros.get("album").map(String::as_str), Some("¡Gulp!"));
        assert_eq!(parametros.get("duration").map(String::as_str), Some("231"));
        assert_eq!(parametros.get("sk").map(String::as_str), Some("sk123"));
    }

    /// Contra Last.fm de verdad, que es lo único que puede decir si el pedido
    /// sale bien armado: la ruta, los parámetros, el `format=json` y —sobre
    /// todo— que un error de la API venga con **código HTTP 200** y el problema
    /// adentro del cuerpo. Mirar el estado HTTP no alcanza, y eso no se puede
    /// comprobar sin salir a la red.
    ///
    /// Con una clave inventada la respuesta es el error 10. La mitad
    /// autenticada —mandar una escucha— necesita una clave real, así que no se
    /// puede probar acá.
    ///
    /// Marcada `ignore`: sale a internet, y una prueba que falla porque no hay
    /// red no dice nada de este código. Se corre a mano con
    /// `cargo test -- --ignored`.
    #[test]
    #[ignore = "sale a la red"]
    fn lastfm_contesta_lo_que_este_codigo_espera() {
        let cliente = Cliente::nuevo(Claves {
            api_key: "clave-inventada".to_string(),
            secreto: "secreto-inventado".to_string(),
        })
        .expect("el cliente se arma");

        let fallo = cliente
            .pedir_token()
            .expect_err("una clave inventada no puede funcionar");

        assert!(
            fallo.contains("(10)"),
            "tendría que ser el error 10 de Last.fm, y fue: {fallo}"
        );
        assert!(
            fallo.contains("Invalid API key"),
            "y tendría que traer el mensaje de allá, y fue: {fallo}"
        );
    }

    /// Un campo vacío en la firma es una firma distinta: si no hay álbum, la
    /// clave no va, no va vacía.
    #[test]
    fn lo_que_no_se_sabe_no_ocupa_lugar() {
        let sin_nada = Escucha {
            album: None,
            duracion_segundos: 0,
            ..escucha()
        };
        let parametros = parametros_de(&sin_nada, &sesion(), true);

        assert_eq!(parametros.get("album"), None);
        assert_eq!(parametros.get("duration"), None);
    }
}
