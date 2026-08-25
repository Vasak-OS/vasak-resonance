//! Los catálogos de idioma tienen que parsear.
//!
//! El plugin de i18n los lee en tiempo de ejecución y **paniquea** si no puede,
//! así que un error de sintaxis no se ve en ninguna compilación: la aplicación
//! simplemente no arranca. Pasó de verdad —un valor sin comillas que contenía
//! `: ` hacía que el parser lo leyera como un mapeo anidado y rompía el archivo
//! entero— y llegó a `main` sin que nada lo detuviera.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn ruta(idioma: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("locales")
        .join(format!("{idioma}.yml"))
}

fn cargar(idioma: &str) -> serde_yaml::Value {
    let texto = std::fs::read_to_string(ruta(idioma))
        .unwrap_or_else(|error| panic!("no se pudo leer {idioma}.yml: {error}"));
    serde_yaml::from_str(&texto)
        .unwrap_or_else(|error| panic!("{idioma}.yml no parsea: {error}"))
}

/// Todas las claves como `grupo.clave`, para poder comparar idiomas.
fn claves(valor: &serde_yaml::Value) -> BTreeSet<String> {
    let mut salida = BTreeSet::new();
    if let serde_yaml::Value::Mapping(grupos) = valor {
        for (grupo, contenido) in grupos {
            let grupo = grupo.as_str().unwrap_or_default();
            match contenido {
                serde_yaml::Value::Mapping(claves) => {
                    for (clave, _) in claves {
                        salida.insert(format!("{grupo}.{}", clave.as_str().unwrap_or_default()));
                    }
                }
                // `_version` y compañía: valores sueltos en la raíz.
                _ => {
                    salida.insert(grupo.to_string());
                }
            }
        }
    }
    salida
}

#[test]
fn los_catalogos_parsean() {
    // Este es el test que faltaba: sin él, un valor con `: ` sin comillas deja
    // la aplicación sin arrancar y no hay nada que lo diga antes.
    cargar("es");
    cargar("en");
}

#[test]
fn los_dos_idiomas_tienen_las_mismas_claves() {
    let es = claves(&cargar("es"));
    let en = claves(&cargar("en"));

    let solo_es: Vec<_> = es.difference(&en).collect();
    let solo_en: Vec<_> = en.difference(&es).collect();

    assert!(solo_es.is_empty(), "claves sólo en español: {solo_es:?}");
    assert!(solo_en.is_empty(), "claves sólo en inglés: {solo_en:?}");
}

#[test]
fn ningun_valor_queda_vacio() {
    // Una clave con el valor vacío se ve como un hueco en la interfaz, que es
    // peor que la clave cruda: al menos la clave se nota.
    for idioma in ["es", "en"] {
        let catalogo = cargar(idioma);
        if let serde_yaml::Value::Mapping(grupos) = catalogo {
            for (grupo, contenido) in grupos {
                if let serde_yaml::Value::Mapping(claves) = contenido {
                    for (clave, valor) in claves {
                        let texto = valor.as_str().unwrap_or_default();
                        assert!(
                            !texto.trim().is_empty(),
                            "{idioma}: {}.{} está vacía",
                            grupo.as_str().unwrap_or_default(),
                            clave.as_str().unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn los_marcadores_de_posicion_coinciden_entre_idiomas() {
    // Si el español dice `{0}` y el inglés no, una de las dos traducciones
    // muestra el marcador crudo o pierde el dato. Es el mismo tipo de error que
    // el `.replace('{0}', …)` que usa este proyecto no puede detectar.
    let es = cargar("es");
    let en = cargar("en");

    let marcadores = |valor: &serde_yaml::Value, ruta: &str| -> Vec<String> {
        let mut salida: Vec<String> = valor
            .as_str()
            .unwrap_or_default()
            .split('{')
            .skip(1)
            .filter_map(|resto| resto.split('}').next())
            .filter(|token| token.chars().all(|c| c.is_ascii_digit()) && !token.is_empty())
            .map(|token| format!("{{{token}}}"))
            .collect();
        salida.sort();
        salida.dedup();
        let _ = ruta;
        salida
    };

    if let (serde_yaml::Value::Mapping(grupos_es), serde_yaml::Value::Mapping(grupos_en)) =
        (&es, &en)
    {
        for (grupo, contenido_es) in grupos_es {
            let Some(contenido_en) = grupos_en.get(grupo) else {
                continue;
            };
            if let (serde_yaml::Value::Mapping(claves_es), serde_yaml::Value::Mapping(claves_en)) =
                (contenido_es, contenido_en)
            {
                for (clave, valor_es) in claves_es {
                    let Some(valor_en) = claves_en.get(clave) else {
                        continue;
                    };
                    let nombre = format!(
                        "{}.{}",
                        grupo.as_str().unwrap_or_default(),
                        clave.as_str().unwrap_or_default()
                    );
                    assert_eq!(
                        marcadores(valor_es, &nombre),
                        marcadores(valor_en, &nombre),
                        "los marcadores de {nombre} no coinciden entre idiomas"
                    );
                }
            }
        }
    }
}
