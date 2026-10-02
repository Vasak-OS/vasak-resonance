//! El mini reproductor como superficie de capa, anclado abajo a la derecha.
//!
//! Tauri sólo sabe crear `xdg_toplevel`, y a un toplevel el compositor lo pone
//! donde quiere: en Wayland una ventana no elige su posición. Para que quede
//! abajo a la derecha hace falta una superficie de `wlr-layer-shell`, y eso
//! tiene dos condiciones que la versión anterior no cumplía:
//!
//! - **La capa se inicializa sobre una ventana que todavía no se mostró
//!   nunca.** Antes se llamaba a `init_layer_shell` sobre la ventana que Tauri
//!   ya había armado desde `tauri.conf.json`. Medido en un Wayfire anidado que
//!   sí da el protocolo: la superficie de capa aparecía anclada en su lugar
//!   pero `mapped: false` y de 0×0 — el mini reproductor no se veía en ningún
//!   lado. Por eso se hace lo mismo que `vasak-desktop` y `vasak-prism`: el
//!   WebView se muda a una `gtk::Window` nueva, que recibe la capa antes de
//!   realizarse, y la de Tauri queda escondida.
//! - **El compositor tiene que ofrecer el protocolo.** En VasakOS lo reparte
//!   `permisos-globales`, sólo a los binarios de su lista. Sin él no hay capa
//!   posible, y el mini reproductor queda como ventana común: ahí lo acomoda
//!   `wayfire_ipc::place_own_view_bottom_right` cada vez que se muestra.

use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;

/// La etiqueta de la ventana en `tauri.conf.json`, que es también lo que la
/// página mira para saber que es el mini reproductor.
pub const MINI_PLAYER_LABEL: &str = "mini-player";

/// El título de la ventana, que es por lo que se la encuentra en Wayfire.
pub const MINI_PLAYER_TITLE: &str = "MiniPlayer - Resonance";

/// Ancho y alto, los mismos de `tauri.conf.json`.
pub const MINI_PLAYER_SIZE: (i32, i32) = (360, 120);

/// La distancia a los bordes de la pantalla.
pub const MINI_PLAYER_MARGIN: i32 = 10;

/// Lo que se le pide al compositor para la superficie de capa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPlacement {
    pub layer: Layer,
    pub namespace: &'static str,
    /// Los bordes a los que se ancla. Los que no están, quedan sueltos.
    pub anchors: Vec<Edge>,
    /// Margen por borde, sólo para los anclados.
    pub margins: Vec<(Edge, i32)>,
    pub size: (i32, i32),
}

/// Encima de las ventanas, abajo a la derecha y a 10 px de cada borde.
pub fn mini_player_placement() -> LayerPlacement {
    LayerPlacement {
        layer: Layer::Top,
        namespace: "vasak-resonance-miniplayer",
        anchors: vec![Edge::Bottom, Edge::Right],
        margins: vec![
            (Edge::Bottom, MINI_PLAYER_MARGIN),
            (Edge::Right, MINI_PLAYER_MARGIN),
        ],
        size: MINI_PLAYER_SIZE,
    }
}

/// Cómo quedó montado el mini reproductor.
enum MiniPlayerSurface {
    /// La superficie de capa, con el WebView adentro.
    Layer(gtk::Window),
    /// La ventana de Tauri tal cual, porque el compositor no dio la capa.
    Toplevel,
}

thread_local! {
    /// Vive en el hilo principal, que es el único donde se puede tocar GTK.
    static MINI_PLAYER: RefCell<Option<MiniPlayerSurface>> = const { RefCell::new(None) };
}

/// Monta el mini reproductor, escondido. Se llama una vez, en `setup`.
///
/// Devuelve `true` si quedó como superficie de capa.
pub fn mount_mini_player(window: &tauri::WebviewWindow) -> Result<bool, String> {
    // Se pregunta antes de tocar nada: sin el protocolo, gtk-layer-shell avisa
    // y sigue, y lo que se le pida después a esa superficie termina en un
    // aborto de libwayland.
    if !gtk_layer_shell::is_supported() {
        eprintln!(
            "[layer_shell] el compositor no ofrece wlr-layer-shell: el mini \
             reproductor va como ventana común y se acomoda por Wayfire"
        );
        MINI_PLAYER.with(|slot| *slot.borrow_mut() = Some(MiniPlayerSurface::Toplevel));
        return Ok(false);
    }

    let frame = window.gtk_window().map_err(|error| error.to_string())?;

    let surface = gtk::Window::new(gtk::WindowType::Toplevel);
    surface.set_decorated(false);
    surface.set_title(MINI_PLAYER_TITLE);
    apply_placement(&surface, &mini_player_placement());

    move_webview(&frame, &surface)?;
    make_transparent(&surface);
    let _ = window.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)));

    // `show_all` realiza los widgets de adentro —el WebView incluido—; se
    // esconde enseguida porque el mini reproductor arranca oculto.
    surface.show_all();
    surface.hide();
    frame.hide();

    MINI_PLAYER.with(|slot| *slot.borrow_mut() = Some(MiniPlayerSurface::Layer(surface)));
    Ok(true)
}

/// Le pone la capa, las anclas y los márgenes. Tiene que ir antes de que la
/// ventana se realice: después, gtk-layer-shell ya no puede cambiar su rol.
fn apply_placement(surface: &gtk::Window, placement: &LayerPlacement) {
    // La capa pide el tamaño a esta ventana: sin pedirlo, se encoge a lo que
    // pida WebKit, que es nada.
    surface.set_size_request(placement.size.0, placement.size.1);

    surface.init_layer_shell();
    surface.set_layer(placement.layer);
    surface.set_namespace(placement.namespace);

    for edge in &placement.anchors {
        surface.set_anchor(*edge, true);
    }
    for (edge, margin) in &placement.margins {
        surface.set_layer_shell_margin(*edge, *margin);
    }
}

/// Saca el WebView de la ventana de Tauri y lo pone en la de capa.
fn move_webview(frame: &gtk::ApplicationWindow, surface: &gtk::Window) -> Result<(), String> {
    let child = frame
        .child()
        .ok_or("la ventana de Tauri del mini reproductor está vacía")?;

    let container = child.dynamic_cast_ref::<gtk::Container>().ok_or_else(|| {
        format!(
            "lo que hay en la ventana de Tauri no es un contenedor: {}",
            child.type_().name()
        )
    })?;

    let webview = container
        .children()
        .first()
        .cloned()
        .ok_or("el contenedor de Tauri no tiene el WebView")?;

    container.remove(&webview);
    surface.add(&webview);
    Ok(())
}

/// Sin esto se dibuja el gris de GTK detrás del WebView, y las esquinas
/// redondeadas quedan sobre un rectángulo opaco.
fn make_transparent(surface: &gtk::Window) {
    if let Some(screen) = WidgetExt::screen(surface) {
        if let Some(rgba) = screen.rgba_visual() {
            surface.set_visual(Some(&rgba));
        }
    }

    let css = gtk::CssProvider::new();
    if css
        .load_from_data(b"window { background-color: rgba(0, 0, 0, 0); }")
        .is_ok()
    {
        surface
            .style_context()
            .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    }
}

/// Si el mini reproductor se ve. `None` si no está montado como capa: ahí la
/// respuesta la tiene la ventana de Tauri.
///
/// Sólo desde el hilo principal.
pub fn layer_visible() -> Option<bool> {
    MINI_PLAYER.with(|slot| match slot.borrow().as_ref() {
        Some(MiniPlayerSurface::Layer(surface)) => Some(surface.is_visible()),
        _ => None,
    })
}

/// Muestra o esconde la superficie de capa. Devuelve `false` si no hay capa,
/// y entonces hay que hacerlo con la ventana de Tauri.
///
/// Esconder y volver a mostrar no pierde la capa: GTK desrealiza la ventana,
/// pero gtk-layer-shell guarda lo pedido y lo vuelve a pedir al mapearla.
///
/// Sólo desde el hilo principal.
pub fn set_layer_visible(visible: bool) -> bool {
    MINI_PLAYER.with(|slot| match slot.borrow().as_ref() {
        Some(MiniPlayerSurface::Layer(surface)) => {
            if visible {
                surface.show_all();
            } else {
                surface.hide();
            }
            true
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_mini_reproductor_se_ancla_abajo_a_la_derecha() {
        let placement = mini_player_placement();
        assert_eq!(placement.anchors, vec![Edge::Bottom, Edge::Right]);
        assert!(!placement.anchors.contains(&Edge::Top));
        assert!(!placement.anchors.contains(&Edge::Left));
    }

    #[test]
    fn deja_diez_pixeles_a_cada_borde_anclado() {
        let placement = mini_player_placement();
        assert_eq!(
            placement.margins,
            vec![(Edge::Bottom, 10), (Edge::Right, 10)]
        );
        for (edge, _) in &placement.margins {
            assert!(
                placement.anchors.contains(edge),
                "margen en un borde suelto"
            );
        }
    }

    #[test]
    fn va_encima_de_las_ventanas_y_con_su_tamano() {
        let placement = mini_player_placement();
        assert_eq!(placement.layer, Layer::Top);
        assert_eq!(placement.namespace, "vasak-resonance-miniplayer");
        assert_eq!(placement.size, (360, 120));
    }

    /// El tamaño y el título tienen que ser los de `tauri.conf.json`: el título
    /// es por lo que Wayfire encuentra la vista, y el tamaño es el que la
    /// página espera.
    #[test]
    fn coincide_con_la_ventana_declarada_en_tauri_conf() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let mini = conf["app"]["windows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|window| window["label"] == MINI_PLAYER_LABEL)
            .expect("tauri.conf.json no declara el mini reproductor");

        assert_eq!(mini["title"], MINI_PLAYER_TITLE);
        assert_eq!(mini["width"], MINI_PLAYER_SIZE.0);
        assert_eq!(mini["height"], MINI_PLAYER_SIZE.1);
        // Arranca escondida: si Tauri la mostrara, se mapearía como toplevel
        // antes de que el WebView se mude a la capa.
        assert_eq!(mini["visible"], false);
    }
}
