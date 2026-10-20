//! Guarda del cableado del afinado del WebKit.
//!
//! El afinado baja al `WebKitWebView` por el árbol de GTK y no se puede ejercitar
//! sin un display ni un WebKit reales, así que vive en `commands/`, que la
//! cobertura excluye. Lo que sí se puede cuidar sin GTK es que las tres puntas
//! sigan conectadas: el comando con sus palancas seguras, su registro en
//! `lib.rs` y la llamada desde el frontend al montar. Si alguna se corta, el
//! afinado deja de aplicarse en silencio —la app abre igual— y esto es lo único
//! que lo detectaría.

use std::path::{Path, PathBuf};

fn read_source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", path.display()))
}

#[test]
fn el_comando_afina_con_las_palancas_seguras() {
    let fuente = read_source("src/commands/webkit.rs");

    assert!(
        fuente.contains("pub fn tune_webview"),
        "falta el comando `tune_webview` en commands/webkit.rs"
    );
    assert!(
        fuente.contains("set_enable_developer_extras(false)"),
        "el afinado tiene que apagar los extras de desarrollo"
    );
    assert!(
        fuente.contains("set_enable_back_forward_navigation_gestures(false)"),
        "el afinado tiene que apagar los gestos de navegación atrás/adelante"
    );
    assert!(
        fuente.contains("CacheModel::DocumentViewer"),
        "el afinado tiene que usar el caché de visor de documento"
    );
}

#[test]
fn el_comando_esta_registrado_en_lib() {
    let fuente = read_source("src/lib.rs");

    assert!(
        fuente.contains("commands::webkit::tune_webview"),
        "`tune_webview` tiene que estar en el `generate_handler!` de lib.rs"
    );
}

#[test]
fn el_frontend_dispara_el_afinado_al_montar() {
    let app_vue: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "src", "App.vue"]
        .iter()
        .collect();
    let fuente = std::fs::read_to_string(&app_vue)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", app_vue.display()));

    assert!(
        fuente.contains("invoke('tune_webview')"),
        "App.vue tiene que llamar a `invoke('tune_webview')` al montar"
    );
}
