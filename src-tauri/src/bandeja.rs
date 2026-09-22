//! El icono de la bandeja del sistema.
//!
//! Habla StatusNotifierItem directo, por `ksni`, que es lo que entiende el
//! panel de VasakOS. Con libappindicator el clic izquierdo no llega nunca: la
//! biblioteca lo consume para abrir el menú y `activate` no se dispara, así que
//! «clic para traer la ventana» queda sin camino de vuelta.

use std::sync::Arc;

use ksni::menu::StandardItem;
use ksni::{Category, MenuItem, Tray};

use crate::mando::Mando;

/// El icono del propio reproductor, el que ya instala el paquete.
///
/// No cambia con el estado a propósito: en la bandeja el icono dice *qué
/// aplicación es*, y el menú dice qué está haciendo. Un icono que alterna entre
/// play y pausa se confunde con un botón y, en un panel con varios elementos,
/// deja de identificar a nadie.
pub const ICONO: &str = "vasak-resonance";

/// Los textos del menú, ya traducidos.
///
/// Se resuelven una vez y se guardan: `menu()` lo llama el panel, que es otro
/// proceso, y buscar en el diccionario de traducciones ahí adentro mete un
/// candado en un camino que tiene que contestar rápido.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextosDelMenu {
    pub mostrar: String,
    pub reproducir: String,
    pub pausar: String,
    pub anterior: String,
    pub siguiente: String,
    pub salir: String,
}

impl Default for TextosDelMenu {
    /// El español, que es el idioma con el que la aplicación venía escrita.
    ///
    /// Es lo que se usa si el plugin de traducciones todavía no está listo o no
    /// encontró sus archivos: un menú en español es peor que uno traducido, y
    /// mucho mejor que uno con las claves crudas.
    fn default() -> Self {
        Self {
            mostrar: "Mostrar".to_string(),
            reproducir: "Reproducir".to_string(),
            pausar: "Pausar".to_string(),
            anterior: "Anterior".to_string(),
            siguiente: "Siguiente".to_string(),
            salir: "Salir".to_string(),
        }
    }
}

/// El icono y su menú.
pub struct Bandeja<M: Mando> {
    mando: Arc<M>,
    textos: TextosDelMenu,
    /// Lo último que se sabe del reproductor, para la etiqueta de
    /// reproducir/pausar. No se pregunta dentro de `menu()` por lo mismo que
    /// los textos.
    sonando: bool,
}

impl<M: Mando> Bandeja<M> {
    pub fn nueva(mando: Arc<M>, textos: TextosDelMenu) -> Self {
        let sonando = mando.esta_sonando();
        Self {
            mando,
            textos,
            sonando,
        }
    }

    /// Anota si está sonando; devuelve si eso cambió algo.
    pub fn anotar_que_suena(&mut self, sonando: bool) -> bool {
        let cambio = self.sonando != sonando;
        self.sonando = sonando;
        cambio
    }

    fn entrada(
        etiqueta: &str,
        icono: &str,
        accion: impl Fn(&M) -> Result<(), String> + Send + 'static,
    ) -> MenuItem<Self> {
        // La etiqueta se copia para el diario: el cierre vive tanto como el
        // menú, y el `&str` que entra es prestado de los textos.
        let nombre = etiqueta.to_string();

        StandardItem {
            label: etiqueta.to_string(),
            icon_name: icono.to_string(),
            activate: Box::new(move |bandeja: &mut Self| {
                // Un fallo acá es una línea en el diario y nada más: el menú de
                // la bandeja no tiene dónde mostrar un error, y cortar la
                // música porque no se pudo enfocar una ventana sería peor.
                if let Err(error) = accion(bandeja.mando.as_ref()) {
                    eprintln!("La bandeja no pudo «{nombre}»: {error}");
                }
            }),
            ..Default::default()
        }
        .into()
    }
}

impl<M: Mando> Tray for Bandeja<M> {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    fn title(&self) -> String {
        "Vasak Resonance".to_string()
    }

    fn icon_name(&self) -> String {
        ICONO.to_string()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    /// El clic izquierdo trae la ventana.
    ///
    /// Es el camino de vuelta, y es la mitad de la razón por la que existe este
    /// icono: una aplicación que se va a la bandeja y no se puede traer es peor
    /// que una que se cierra.
    fn activate(&mut self, _x: i32, _y: i32) {
        if let Err(error) = self.mando.mostrar() {
            eprintln!("La bandeja no pudo traer la ventana: {error}");
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let (etiqueta_de_sonar, icono_de_sonar) = if self.sonando {
            (&self.textos.pausar, "media-playback-pause")
        } else {
            (&self.textos.reproducir, "media-playback-start")
        };

        vec![
            Self::entrada(&self.textos.mostrar, "window", |mando| mando.mostrar()),
            MenuItem::Separator,
            Self::entrada(etiqueta_de_sonar, icono_de_sonar, |mando| {
                mando.reproducir_o_pausar()
            }),
            Self::entrada(&self.textos.anterior, "media-skip-backward", |mando| {
                mando.anterior()
            }),
            Self::entrada(&self.textos.siguiente, "media-skip-forward", |mando| {
                mando.siguiente()
            }),
            MenuItem::Separator,
            StandardItem {
                label: self.textos.salir.clone(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(|bandeja: &mut Self| bandeja.mando.salir()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Qué avisarle al icono ante un estado nuevo, según lo último que se le avisó.
///
/// `None` significa no tocarlo. Está aparte y sin nada de Tauri adentro porque
/// es la única parte de este archivo que decide algo, y el resto del arranque
/// —un `spawn` de D-Bus y un bucle de eventos— no se puede probar.
fn proximo_aviso(ultimo_avisado: Option<bool>, sonando: bool) -> Option<bool> {
    match ultimo_avisado {
        Some(anterior) if anterior == sonando => None,
        _ => Some(sonando),
    }
}

/// Levanta el icono y lo deja al día mientras la aplicación viva.
///
/// No corta el arranque si falla: una sesión sin StatusNotifierItem —o con el
/// panel todavía sin levantar— deja al reproductor sin icono, que es molesto,
/// pero la aplicación abre igual.
pub fn iniciar(app_handle: tauri::AppHandle, audio_state: crate::audio_manager::AudioState) {
    use ksni::TrayMethods;
    use tauri::Listener;

    use crate::mando::MandoDeTauri;

    let mando = Arc::new(MandoDeTauri::nuevo(app_handle.clone(), audio_state));

    tauri::async_runtime::spawn(async move {
        let bandeja = Bandeja::nueva(mando.clone(), textos_del_menu(&app_handle));
        let manija = match bandeja.spawn().await {
            Ok(manija) => manija,
            Err(error) => {
                eprintln!("La bandeja no pudo registrarse: {error}");
                return;
            }
        };

        // El hilo de audio avisa cada 500 ms, no sólo cuando algo cambia. Cada
        // `update` hace que el panel vuelva a pedir el menú entero, así que se
        // toca sólo cuando el estado cambió de verdad.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        app_handle.listen("audio-playback-progress", move |_| {
            let _ = tx.send(());
        });

        // Lo que el icono ya sabe, que arranca sin saber nada. `Bandeja::nueva`
        // miró el reproductor **antes** de `spawn`, y en el medio hay un viaje
        // a D-Bus: si la música arrancó ahí —que es justo lo que pasa cuando el
        // gestor de archivos abre un tema— el icono quedó con la etiqueta de
        // antes y ningún aviso posterior la corrige, porque el estado ya no
        // vuelve a cambiar. Empezar sin saber nada hace que el primer aviso
        // salga siempre.
        let mut avisado: Option<bool> = None;

        loop {
            if let Some(sonando) = proximo_aviso(avisado, mando.esta_sonando()) {
                avisado = Some(sonando);
                manija
                    .update(move |bandeja: &mut Bandeja<MandoDeTauri>| {
                        bandeja.anotar_que_suena(sonando);
                    })
                    .await;
            }

            if rx.recv().await.is_none() {
                break;
            }
        }
    });
}

/// Los textos del menú en el idioma de la sesión.
///
/// Se piden con `try_state` y no con el ayudante del plugin: ése entra por
/// `state()`, que entra en pánico si el plugin no llegó a registrarse, y esto
/// corre en el arranque. Sin traducciones, el menú sale en español.
fn textos_del_menu(app_handle: &tauri::AppHandle) -> TextosDelMenu {
    use tauri::Manager;

    let Some(i18n) = app_handle.try_state::<tauri_plugin_i18n_vsk::PluginI18n<tauri::Wry>>() else {
        return TextosDelMenu::default();
    };

    let por_omision = TextosDelMenu::default();
    let traducir = |clave: &str, si_falta: &str| {
        i18n.translate(clave)
            .map(str::to_string)
            .unwrap_or_else(|| si_falta.to_string())
    };

    TextosDelMenu {
        mostrar: traducir("tray.show", &por_omision.mostrar),
        reproducir: traducir("tray.play", &por_omision.reproducir),
        pausar: traducir("tray.pause", &por_omision.pausar),
        anterior: traducir("tray.previous", &por_omision.anterior),
        siguiente: traducir("tray.next", &por_omision.siguiente),
        salir: traducir("tray.quit", &por_omision.salir),
    }
}

#[cfg(test)]
mod pruebas {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MandoDePrueba {
        pedido: Mutex<Vec<&'static str>>,
        sonando: bool,
        falla: bool,
    }

    impl MandoDePrueba {
        fn anotar(&self, que: &'static str) -> Result<(), String> {
            self.pedido.lock().expect("el candado").push(que);
            if self.falla {
                Err("falla a propósito".to_string())
            } else {
                Ok(())
            }
        }

        fn lo_pedido(&self) -> Vec<&'static str> {
            self.pedido.lock().expect("el candado").clone()
        }
    }

    impl Mando for MandoDePrueba {
        fn reproducir_o_pausar(&self) -> Result<(), String> {
            self.anotar("reproducir_o_pausar")
        }

        fn siguiente(&self) -> Result<(), String> {
            self.anotar("siguiente")
        }

        fn anterior(&self) -> Result<(), String> {
            self.anotar("anterior")
        }

        fn mostrar(&self) -> Result<(), String> {
            self.anotar("mostrar")
        }

        fn salir(&self) {
            let _ = self.anotar("salir");
        }

        fn esta_sonando(&self) -> bool {
            self.sonando
        }
    }

    fn bandeja_de_prueba(mando: MandoDePrueba) -> (Arc<MandoDePrueba>, Bandeja<MandoDePrueba>) {
        let mando = Arc::new(mando);
        let bandeja = Bandeja::nueva(mando.clone(), TextosDelMenu::default());
        (mando, bandeja)
    }

    fn etiquetas(bandeja: &Bandeja<MandoDePrueba>) -> Vec<String> {
        bandeja
            .menu()
            .into_iter()
            .map(|entrada| match entrada {
                MenuItem::Standard(normal) => normal.label,
                MenuItem::Separator => "—".to_string(),
                _ => "otra cosa".to_string(),
            })
            .collect()
    }

    /// Activa la entrada que está en `indice`, como haría un clic en el panel.
    fn activar(bandeja: &mut Bandeja<MandoDePrueba>, indice: usize) {
        let entrada = bandeja
            .menu()
            .into_iter()
            .nth(indice)
            .expect("el menú tiene esa entrada");

        let MenuItem::Standard(normal) = entrada else {
            panic!("la entrada {indice} no es una entrada normal");
        };

        (normal.activate)(bandeja);
    }

    #[test]
    fn el_menu_tiene_las_acciones_en_orden() {
        let (_mando, bandeja) = bandeja_de_prueba(MandoDePrueba::default());

        assert_eq!(
            etiquetas(&bandeja),
            vec![
                "Mostrar",
                "—",
                "Reproducir",
                "Anterior",
                "Siguiente",
                "—",
                "Salir",
            ]
        );
    }

    #[test]
    fn la_entrada_dice_pausar_cuando_algo_suena() {
        let (_mando, bandeja) = bandeja_de_prueba(MandoDePrueba {
            sonando: true,
            ..Default::default()
        });

        assert!(etiquetas(&bandeja).contains(&"Pausar".to_string()));
        assert!(!etiquetas(&bandeja).contains(&"Reproducir".to_string()));
    }

    #[test]
    fn anotar_que_suena_avisa_solo_cuando_cambia() {
        let (_mando, mut bandeja) = bandeja_de_prueba(MandoDePrueba::default());

        assert!(bandeja.anotar_que_suena(true), "de pausa a sonando cambia");
        assert!(!bandeja.anotar_que_suena(true), "de sonando a sonando, no");
        assert!(bandeja.anotar_que_suena(false), "y de vuelta, sí");
    }

    #[test]
    fn la_etiqueta_sigue_a_lo_que_se_anota() {
        let (_mando, mut bandeja) = bandeja_de_prueba(MandoDePrueba::default());
        assert!(etiquetas(&bandeja).contains(&"Reproducir".to_string()));

        bandeja.anotar_que_suena(true);

        assert!(etiquetas(&bandeja).contains(&"Pausar".to_string()));
    }

    /// Que cada entrada llame a *lo suyo*: es exactamente lo que se cruza al
    /// escribirlas, y un menú con «Siguiente» que vuelve atrás pasa
    /// desapercibido hasta que alguien lo usa.
    #[test]
    fn cada_entrada_llama_a_su_accion() {
        let (mando, mut bandeja) = bandeja_de_prueba(MandoDePrueba::default());

        for indice in [0, 2, 3, 4, 6] {
            activar(&mut bandeja, indice);
        }

        assert_eq!(
            mando.lo_pedido(),
            vec![
                "mostrar",
                "reproducir_o_pausar",
                "anterior",
                "siguiente",
                "salir",
            ]
        );
    }

    /// El camino de vuelta. Sin esto la bandeja es una calle de una sola mano.
    #[test]
    fn el_clic_izquierdo_trae_la_ventana() {
        let (mando, mut bandeja) = bandeja_de_prueba(MandoDePrueba::default());

        bandeja.activate(0, 0);

        assert_eq!(mando.lo_pedido(), vec!["mostrar"]);
    }

    #[test]
    fn un_fallo_del_mando_no_voltea_la_bandeja() {
        let (mando, mut bandeja) = bandeja_de_prueba(MandoDePrueba {
            falla: true,
            ..Default::default()
        });

        bandeja.activate(0, 0);
        for indice in [0, 2, 3, 4, 6] {
            activar(&mut bandeja, indice);
        }

        assert_eq!(mando.lo_pedido().len(), 6, "se intentaron las seis");
    }

    #[test]
    fn los_textos_traducidos_llegan_al_menu() {
        let mando = Arc::new(MandoDePrueba::default());
        let bandeja = Bandeja::nueva(
            mando,
            TextosDelMenu {
                mostrar: "Show".to_string(),
                reproducir: "Play".to_string(),
                pausar: "Pause".to_string(),
                anterior: "Previous".to_string(),
                siguiente: "Next".to_string(),
                salir: "Quit".to_string(),
            },
        );

        assert_eq!(
            etiquetas(&bandeja),
            vec!["Show", "—", "Play", "Previous", "Next", "—", "Quit"]
        );
    }

    /// El primer aviso sale siempre, aunque coincida con lo que la bandeja
    /// creía. Es lo que arregla arrancar con un tema: entre construir la
    /// bandeja y registrarla en D-Bus la música ya empezó, y sin esto el menú
    /// se queda diciendo «Reproducir» hasta el próximo cambio de estado.
    #[test]
    fn el_primer_aviso_sale_siempre() {
        assert_eq!(proximo_aviso(None, true), Some(true));
        assert_eq!(proximo_aviso(None, false), Some(false));
    }

    /// Y no se repite: el hilo de audio avisa cada 500 ms, y cada aviso hace
    /// que el panel vuelva a pedir el menú entero.
    #[test]
    fn lo_ya_avisado_no_se_repite() {
        assert_eq!(proximo_aviso(Some(true), true), None);
        assert_eq!(proximo_aviso(Some(false), false), None);
    }

    #[test]
    fn el_cambio_si_se_avisa() {
        assert_eq!(proximo_aviso(Some(true), false), Some(false));
        assert_eq!(proximo_aviso(Some(false), true), Some(true));
    }

    /// El identificador tiene que ser el mismo entre sesiones: el panel lo usa
    /// para acordarse de dónde puso el icono.
    #[test]
    fn el_identificador_es_el_nombre_del_paquete() {
        let (_mando, bandeja) = bandeja_de_prueba(MandoDePrueba::default());

        assert_eq!(bandeja.id(), "vasak-resonance");
    }
}
