//! El directorio de emisoras de Internet, que es Radio Browser.
//!
//! Dos cosas que no son detalles. La primera es que **los servidores se
//! descubren**: Radio Browser es una red de réplicas que cambia, y tener tres
//! escritas a mano quiere decir que el día que esas tres no contesten no hay
//! radio aunque la red tenga otras.
//!
//! La segunda es que **lo que devuelve el directorio lo carga cualquiera**: es
//! abierto. De ahí sale la dirección que se le pasa al reproductor, la que
//! termina en una etiqueta `img` de la ventana y la que puede abrirse en un
//! navegador. Que sean direcciones `http(s)` y nada más es la comprobación que
//! conviene tener antes de que haga falta.

use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// De acá sale la lista de réplicas.
const DESCUBRIMIENTO: &str = "https://all.api.radio-browser.info/json/servers";

/// Las que se usan si el descubrimiento no contesta.
const SERVIDORES_DE_RESPALDO: [&str; 3] = [
    "https://de1.api.radio-browser.info",
    "https://nl1.api.radio-browser.info",
    "https://at1.api.radio-browser.info",
];

/// Radio Browser pide que quien consulta se identifique.
const USER_AGENT: &str = concat!("vasak-resonance/", env!("CARGO_PKG_VERSION"));

/// Cuánto se espera a cada servidor antes de probar el siguiente.
const ESPERA: Duration = Duration::from_secs(10);

/// Cuántas emisoras se piden. El directorio tiene decenas de miles y la vista
/// no las necesita todas.
const LIMITE: usize = 100;

/// A cuántas réplicas se le pregunta antes de darse por vencido.
///
/// El descubrimiento puede anunciar muchas, y cada intento espera hasta diez
/// segundos: sin techo, un directorio con todas las réplicas caídas dejaría la
/// vista esperando minutos. Cuatro son más que las tres de antes y acotan la
/// espera a menos de un minuto.
const MAXIMO_REPLICAS: usize = 4;

/// Hasta dónde se leen los textos que vienen del directorio.
const LARGO_NOMBRE: usize = 200;
/// Hasta dónde puede medir una dirección.
///
/// Las direcciones **no se recortan**: una recortada sigue pareciendo una
/// dirección válida —mismo esquema, mismo host— pero apunta a otra cosa, así que
/// pasaría la comprobación y llegaría a la ventana rota. O entera o ninguna.
const LARGO_DIRECCION: usize = 2048;
const LARGO_PAIS: usize = 100;
const LARGO_ETIQUETAS: usize = 512;
const LARGO_CODEC: usize = 32;

/// Una emisora, tal como la devuelve Radio Browser.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RadioStation {
    /// Sólo al **leer** del directorio, que lo llama `stationuuid`.
    ///
    /// Con `rename` a secas el renombre vale para los dos lados, así que la
    /// ventana recibía `stationuuid` mientras su tipo declaraba `uuid`: el campo
    /// llegaba siempre vacío y nadie se enteraba. De ahí salían el icono roto de
    /// una emisora escondiendo el de todas —el conjunto se indexa por uuid— y el
    /// indicador de carga sin saber de cuál era.
    #[serde(rename(deserialize = "stationuuid"))]
    pub uuid: String,
    pub name: String,
    pub url: String,
    /// La dirección que el directorio ya siguió hasta el stream de verdad.
    ///
    /// No se le manda a la ventana: se usa acá para reemplazar `url` cuando
    /// sirve, que es lo que evita una redirección de más en cada reproducción.
    #[serde(default, skip_serializing)]
    pub url_resolved: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub favicon: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub votes: Option<u32>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub bitrate: Option<u32>,
}

/// Si una dirección es de las que se pueden usar.
///
/// `http` además de `https` porque media radio del mundo sigue emitiendo sin
/// cifrar; lo que se descarta es todo lo demás —`file:`, `javascript:`, una
/// ruta local— y los nombres de host que no puedan ser de Internet.
fn direccion_utilizable(url: &str) -> bool {
    let Ok(parseada) = reqwest::Url::parse(url) else {
        return false;
    };

    if !matches!(parseada.scheme(), "http" | "https") {
        return false;
    }

    // Un host de una sola etiqueta —`localhost`, el nombre de una máquina de la
    // red— no es una emisora de Internet.
    parseada.host_str().is_some_and(|host| host.contains('.'))
}

/// Recorta un texto del directorio: sin caracteres de control y con un techo.
fn limpiar(valor: &str, largo: usize) -> String {
    valor
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(largo)
        .collect()
}

fn limpiar_opcional(valor: Option<String>, largo: usize) -> Option<String> {
    let limpio = limpiar(valor.unwrap_or_default().as_str(), largo);
    if limpio.is_empty() {
        None
    } else {
        Some(limpio)
    }
}

/// Deja sólo direcciones utilizables, o nada.
///
/// Se le pasa el valor **sin recortar**: recortar una dirección la deja
/// pareciendo válida y apuntando a otro lado.
fn solo_si_es_una_direccion(valor: Option<String>) -> Option<String> {
    valor
        .map(|url| url.trim().to_string())
        .filter(|url| url.len() <= LARGO_DIRECCION)
        .filter(|url| direccion_utilizable(url))
}

/// El identificador que usa Radio Browser: un UUID con sus guiones.
fn uuid_valido(uuid: &str) -> bool {
    uuid.len() == 36 && uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Pone en orden lo que devolvió el directorio, o la descarta.
///
/// Se descarta la emisora sin identificador, sin nombre o sin una dirección que
/// se pueda reproducir. Lo demás se limpia: `homepage` y `favicon` que no sean
/// direcciones utilizables se vacían en vez de viajar a la ventana, que es
/// donde uno termina en un `img` y el otro en un navegador.
pub fn normalizar(mut estacion: RadioStation) -> Option<RadioStation> {
    estacion.uuid = estacion.uuid.trim().to_string();
    estacion.name = limpiar(&estacion.name, LARGO_NOMBRE);
    estacion.url = estacion.url.trim().to_string();
    estacion.url_resolved = estacion.url_resolved.trim().to_string();

    if !uuid_valido(&estacion.uuid) || estacion.name.is_empty() {
        return None;
    }

    // La resuelta primero: es la que el directorio ya siguió hasta el stream.
    if direccion_utilizable(&estacion.url_resolved) {
        estacion.url = estacion.url_resolved.clone();
    } else if !direccion_utilizable(&estacion.url) {
        return None;
    }

    estacion.homepage = solo_si_es_una_direccion(estacion.homepage);
    estacion.favicon = solo_si_es_una_direccion(estacion.favicon);
    estacion.country = limpiar_opcional(estacion.country, LARGO_PAIS);
    estacion.state = limpiar_opcional(estacion.state, LARGO_PAIS);
    estacion.language = limpiar_opcional(estacion.language, LARGO_PAIS);
    estacion.tags = limpiar_opcional(estacion.tags, LARGO_ETIQUETAS);
    estacion.codec = limpiar_opcional(estacion.codec, LARGO_CODEC).map(|c| c.to_ascii_uppercase());

    Some(estacion)
}

fn cliente() -> &'static reqwest::Client {
    static CLIENTE: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENTE.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(ESPERA)
            .build()
            .unwrap_or_default()
    })
}

#[derive(Deserialize)]
struct ServidorDelDirectorio {
    name: String,
}

/// Las réplicas anunciadas más las de respaldo, sin repetir y en ese orden.
///
/// El directorio anuncia **una entrada por dirección IP**, así que una réplica
/// con IPv4 e IPv6 viene dos veces con el mismo nombre. Sin deduplicar, cuando
/// una no contesta se la vuelve a probar en vez de pasar a la siguiente.
fn sin_repetidos(anunciados: Vec<String>) -> Vec<String> {
    let mut lista: Vec<String> =
        Vec::with_capacity(anunciados.len() + SERVIDORES_DE_RESPALDO.len());

    for servidor in anunciados
        .into_iter()
        .chain(SERVIDORES_DE_RESPALDO.iter().map(|s| s.to_string()))
    {
        if !lista.contains(&servidor) {
            lista.push(servidor);
        }
    }

    lista
}

/// Las réplicas a las que se le puede preguntar, descubiertas una vez por
/// sesión.
///
/// Si el descubrimiento no contesta quedan las de respaldo, que es el
/// comportamiento de antes: no se pierde nada por intentarlo.
async fn servidores() -> Vec<String> {
    static SERVIDORES: OnceLock<Vec<String>> = OnceLock::new();

    if let Some(ya_descubiertos) = SERVIDORES.get() {
        return ya_descubiertos.clone();
    }

    let descubiertos = cliente()
        .get(DESCUBRIMIENTO)
        .send()
        .await
        .ok()
        .filter(|respuesta| respuesta.status().is_success());

    let anunciados: Vec<String> = match descubiertos {
        Some(respuesta) => respuesta
            .json::<Vec<ServidorDelDirectorio>>()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|servidor| format!("https://{}", servidor.name.trim()))
            .filter(|url| direccion_utilizable(url))
            .collect(),
        None => Vec::new(),
    };

    let lista = sin_repetidos(anunciados);

    // Si otra llamada ganó la carrera, `set` falla y vale la lista de ella: son
    // la misma consulta, y lo que importa es descubrir una sola vez por sesión.
    let _ = SERVIDORES.set(lista);
    SERVIDORES.get().cloned().unwrap_or_default()
}

/// Busca emisoras por etiqueta, probando las réplicas hasta que una conteste.
pub async fn fetch_stations(tags: Vec<&str>) -> Result<Vec<RadioStation>, String> {
    let etiqueta = if tags.is_empty() {
        "music".to_string()
    } else {
        tags.join(",")
    };
    let etiqueta = urlencoding::encode(&etiqueta).into_owned();

    let mut ultimo_error = String::from("no se pudo preguntarle a ninguna réplica");

    for servidor in servidores().await.into_iter().take(MAXIMO_REPLICAS) {
        match pedir_estaciones(&url_de_busqueda(&servidor, &etiqueta)).await {
            Ok(estaciones) if !estaciones.is_empty() => return Ok(estaciones),
            Ok(_) => ultimo_error = format!("{servidor} no devolvió ninguna emisora utilizable"),
            Err(error) => ultimo_error = format!("{servidor}: {error}"),
        }
    }

    Err(format!(
        "No se pudo consultar el directorio de radios ({ultimo_error})"
    ))
}

/// La consulta de búsqueda contra una réplica.
///
/// `tagList` y no `tag`: el primero es el que Radio Browser define para una
/// lista separada por comas. Con `tag`, «rock,jazz» se compara como **texto**
/// contra la lista de etiquetas de cada emisora, así que sólo encuentra las que
/// tengan esas dos justo una al lado de la otra. Con una etiqueta sola los dos
/// devuelven lo mismo —comprobado contra el directorio—, así que el cambio no
/// altera lo que se ve hoy.
fn url_de_busqueda(servidor: &str, etiqueta: &str) -> String {
    format!(
        "{servidor}/json/stations/search?tagList={etiqueta}&hidebroken=true&order=votes&reverse=true&limit={LIMITE}"
    )
}

async fn pedir_estaciones(url: &str) -> Result<Vec<RadioStation>, String> {
    let respuesta = cliente()
        .get(url)
        .send()
        .await
        .map_err(|error| format!("no contestó ({error})"))?;

    let estado = respuesta.status();
    if !estado.is_success() {
        return Err(format!("contestó {}", estado.as_u16()));
    }

    let crudas: Vec<RadioStation> = respuesta
        .json()
        .await
        .map_err(|error| format!("contestó algo que no se pudo leer ({error})"))?;

    Ok(crudas.into_iter().filter_map(normalizar).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El nombre del campo **en los dos sentidos**, que no es el mismo.
    ///
    /// El directorio lo llama `stationuuid` y la ventana lo espera como `uuid`.
    /// Con un `rename` a secas el renombre valía para los dos lados y la ventana
    /// recibía un campo que su tipo no declara: `station.uuid` era `undefined`
    /// en las cincuenta y tres emisoras, sin un solo error. Lo que se veía era
    /// otra cosa —un icono roto escondía el de todas, porque el conjunto de
    /// «este icono no carga» se indexa por uuid— y costaba llegar hasta acá.
    #[test]
    fn el_uuid_entra_como_stationuuid_y_sale_como_uuid() {
        let del_directorio = r#"{
            "stationuuid": "9617a958-0601-11e8-ae97-52543be04c81",
            "name": "Una emisora",
            "url": "https://ejemplo/stream"
        }"#;

        let leida: RadioStation = serde_json::from_str(del_directorio).expect("se lee");
        assert_eq!(leida.uuid, "9617a958-0601-11e8-ae97-52543be04c81");

        let hacia_la_ventana = serde_json::to_value(&leida).expect("se serializa");
        assert_eq!(
            hacia_la_ventana.get("uuid").and_then(|v| v.as_str()),
            Some("9617a958-0601-11e8-ae97-52543be04c81"),
            "la ventana lo espera como `uuid`"
        );
        assert!(
            hacia_la_ventana.get("stationuuid").is_none(),
            "y no como `stationuuid`, que es el nombre del directorio"
        );
    }

    fn estacion() -> RadioStation {
        RadioStation {
            uuid: "9617a958-0601-11e8-ae97-52543be04c81".into(),
            name: "Una emisora".into(),
            url: "http://emisora.ejemplo/stream".into(),
            url_resolved: String::new(),
            homepage: None,
            favicon: None,
            tags: None,
            country: None,
            state: None,
            language: None,
            votes: None,
            codec: None,
            bitrate: None,
        }
    }

    #[test]
    fn una_emisora_normal_pasa() {
        assert!(normalizar(estacion()).is_some());
    }

    #[test]
    fn sin_identificador_o_sin_nombre_no_pasa() {
        let mut sin_uuid = estacion();
        sin_uuid.uuid = "no-es-un-uuid".into();
        assert!(normalizar(sin_uuid).is_none());

        let mut sin_nombre = estacion();
        sin_nombre.name = "   ".into();
        assert!(normalizar(sin_nombre).is_none());
    }

    #[test]
    fn se_prefiere_la_direccion_que_el_directorio_ya_resolvio() {
        // Es la que evita una redirección en cada reproducción.
        let mut con_resuelta = estacion();
        con_resuelta.url_resolved = "https://cdn.ejemplo/stream.mp3".into();

        let normalizada = normalizar(con_resuelta).expect("tendría que pasar");

        assert_eq!(normalizada.url, "https://cdn.ejemplo/stream.mp3");
    }

    #[test]
    fn una_resuelta_que_no_sirve_no_pisa_a_la_buena() {
        let mut con_resuelta_mala = estacion();
        con_resuelta_mala.url_resolved = "file:///etc/passwd".into();

        let normalizada = normalizar(con_resuelta_mala).expect("tendría que pasar igual");

        assert_eq!(normalizada.url, "http://emisora.ejemplo/stream");
    }

    #[test]
    fn una_emisora_sin_direccion_utilizable_se_descarta() {
        // Esta es la que termina en el reproductor.
        for direccion in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "no es una dirección",
            "http://localhost/stream",
            "",
        ] {
            let mut mala = estacion();
            mala.url = direccion.into();
            assert!(normalizar(mala).is_none(), "{direccion}");
        }
    }

    #[test]
    fn el_icono_y_la_pagina_que_no_sirven_se_vacian_en_vez_de_viajar() {
        // Uno termina en un `img` de la ventana y el otro puede abrirse en un
        // navegador; que la emisora sea buena no los hace buenos a ellos.
        let mut con_basura = estacion();
        con_basura.favicon = Some("javascript:alert(1)".into());
        con_basura.homepage = Some("file:///home/alguien".into());

        let normalizada = normalizar(con_basura).expect("la emisora sirve igual");

        assert_eq!(normalizada.favicon, None);
        assert_eq!(normalizada.homepage, None);
    }

    #[test]
    fn los_caracteres_de_control_no_pasan() {
        // Van a una lista de la ventana; un salto de línea adentro del nombre
        // rompe cualquier cosa que lo muestre.
        let mut con_control = estacion();
        con_control.name = "Radio\u{0}\nCon basura".into();

        let normalizada = normalizar(con_control).expect("tendría que pasar");

        assert_eq!(normalizada.name, "RadioCon basura");
    }

    #[test]
    fn los_textos_larguisimos_se_recortan() {
        let mut larguisima = estacion();
        larguisima.name = "a".repeat(5000);

        let normalizada = normalizar(larguisima).expect("tendría que pasar");

        assert_eq!(normalizada.name.chars().count(), LARGO_NOMBRE);
    }

    #[test]
    fn una_direccion_larguisima_se_descarta_en_vez_de_recortarse() {
        // Recortada seguiría pareciendo una dirección válida —mismo esquema,
        // mismo host— y llegaría a la ventana apuntando a otra cosa: una imagen
        // rota o un enlace equivocado, en vez de un campo vacío.
        let mut con_favicon_larguisimo = estacion();
        let larga = format!("https://emisora.ejemplo/{}", "a".repeat(LARGO_DIRECCION));
        con_favicon_larguisimo.favicon = Some(larga);

        let normalizada = normalizar(con_favicon_larguisimo).expect("la emisora sirve igual");

        assert_eq!(normalizada.favicon, None);
    }

    #[test]
    fn una_direccion_de_largo_normal_pasa_entera() {
        let mut con_favicon = estacion();
        let url = "https://emisora.ejemplo/icono.png?version=2&tamano=128";
        con_favicon.favicon = Some(url.into());

        let normalizada = normalizar(con_favicon).expect("pasa");

        assert_eq!(normalizada.favicon.as_deref(), Some(url));
    }

    #[test]
    fn la_busqueda_usa_el_parametro_de_lista_de_etiquetas() {
        // Con `tag`, «rock,jazz» se compara como texto contra la lista de
        // etiquetas de cada emisora, así que sólo encuentra las que tengan esas
        // dos justo una al lado de la otra.
        let url = url_de_busqueda("https://de1.api.radio-browser.info", "rock%2Cjazz");

        assert!(url.contains("tagList=rock%2Cjazz"), "{url}");
        assert!(!url.contains("?tag="), "{url}");
    }

    #[test]
    fn la_lista_de_servidores_no_repite() {
        // El directorio anuncia una entrada por dirección IP, así que una
        // réplica con IPv4 e IPv6 viene dos veces con el mismo nombre. Sin
        // deduplicar, cuando una no contesta se la vuelve a probar en vez de
        // pasar a la siguiente. Salió al probar contra el directorio de verdad.
        let anunciados = vec![
            "https://de1.api.radio-browser.info".to_string(),
            "https://de1.api.radio-browser.info".to_string(),
            "https://nl1.api.radio-browser.info".to_string(),
        ];

        let lista = sin_repetidos(anunciados);

        assert_eq!(
            lista,
            vec![
                "https://de1.api.radio-browser.info",
                "https://nl1.api.radio-browser.info",
                "https://at1.api.radio-browser.info",
            ]
        );
    }

    #[test]
    fn los_de_respaldo_estan_siempre_y_al_final() {
        let lista = sin_repetidos(vec!["https://otra.api.radio-browser.info".to_string()]);

        assert_eq!(lista[0], "https://otra.api.radio-browser.info");
        for respaldo in SERVIDORES_DE_RESPALDO {
            assert!(lista.contains(&respaldo.to_string()), "{respaldo}");
        }
    }

    #[test]
    fn el_codec_queda_en_mayusculas() {
        let mut con_codec = estacion();
        con_codec.codec = Some("mp3".into());

        assert_eq!(
            normalizar(con_codec).expect("pasa").codec.as_deref(),
            Some("MP3")
        );
    }
}
