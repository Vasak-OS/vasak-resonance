//! Mostrar una canción en el gestor de archivos.
//!
//! Se abre la carpeta que contiene el archivo, no el archivo: abrirlo lo
//! mandaría al reproductor por omisión, que es esta misma aplicación, y el clic
//! derecho ya ofrece reproducirlo.
//!
//! Va por `xdg-open` sobre el directorio, así que quien atiende es el gestor de
//! archivos elegido para `inode/directory` — en VasakOS, vasak-file-manager. No
//! se usa `org.freedesktop.FileManager1.ShowItems`, que además seleccionaría el
//! archivo, porque activa por D-Bus a quien haya reclamado ese nombre en el
//! sistema y eso puede ser un gestor de archivos de otro escritorio, ignorando
//! la elección del usuario.
//!
//! Tampoco se usa el comando `open` del plugin de shell: llamado desde la
//! página exige abrir el alcance a rutas arbitrarias del disco, y esto sólo
//! necesita abrir una carpeta que ya está en la biblioteca.

use std::path::Path;
use std::process::{Command, Stdio};

use serde::Serialize;

/// Por qué no se pudo mostrar la canción.
///
/// La interfaz decide qué decir a partir de esto y no del texto: así el aviso
/// está traducido y distingue «la carpeta no se abrió» de «la canción ya no
/// está», que para quien lo lee no son lo mismo.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RevealErrorKind {
    /// La ruta guardada no tiene carpeta contenedora.
    NoContainingFolder,
    /// La carpeta ya no existe.
    FolderMissing,
    /// La carpeta sigue ahí, pero el archivo de la canción no.
    FileMissing,
    /// No se pudo lanzar el gestor de archivos.
    LaunchFailed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealError {
    pub kind: RevealErrorKind,
    /// Detalle con la ruta, para el registro de la aplicación.
    pub detail: String,
}

impl RevealError {
    fn new(kind: RevealErrorKind, detail: String) -> Self {
        Self { kind, detail }
    }
}

#[tauri::command]
pub fn show_in_file_manager(path: String) -> Result<(), RevealError> {
    let file = Path::new(&path);

    let folder = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            RevealError::new(
                RevealErrorKind::NoContainingFolder,
                format!("La ruta no tiene carpeta contenedora: {path}"),
            )
        })?;

    if !folder.is_dir() {
        return Err(RevealError::new(
            RevealErrorKind::FolderMissing,
            format!(
                "La carpeta de la canción ya no existe: {}",
                folder.display()
            ),
        ));
    }

    // La biblioteca recuerda canciones que se pueden haber borrado o movido
    // desde fuera del reproductor. Sin esta comprobación se abría la carpeta y
    // se contestaba que todo salió bien, así que quien pidió «mostrar en el
    // gestor de archivos» se quedaba mirando una carpeta sin la canción, sin
    // que nadie le dijera que ya no está.
    if !file.exists() {
        return Err(RevealError::new(
            RevealErrorKind::FileMissing,
            format!("El archivo de la canción ya no existe: {path}"),
        ));
    }

    // Sin heredar las tuberías: el gestor de archivos sobrevive a esta ventana
    // y no tiene por qué escribir en la salida del reproductor.
    let mut child = Command::new("xdg-open")
        .arg(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            RevealError::new(
                RevealErrorKind::LaunchFailed,
                format!("No se pudo abrir el gestor de archivos: {error}"),
            )
        })?;

    // `xdg-open` delega y termina enseguida. Sin nadie que lo espere quedaría un
    // proceso zombi por cada vez que se usa el menú, y el reproductor es una
    // ventana que se deja abierta todo el día.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nada de esto llega a lanzar `xdg-open`: las comprobaciones cortan antes,
    /// que es justamente lo que se está probando.
    fn kind_of(path: &str) -> RevealErrorKind {
        show_in_file_manager(path.to_string())
            .expect_err("se esperaba un error")
            .kind
    }

    #[test]
    fn una_ruta_sin_carpeta_no_abre_nada() {
        assert_eq!(kind_of(""), RevealErrorKind::NoContainingFolder);
    }

    #[test]
    fn una_carpeta_que_ya_no_existe_se_avisa() {
        assert_eq!(
            kind_of("/carpeta/que/no/existe/cancion.mp3"),
            RevealErrorKind::FolderMissing
        );
    }

    /// El caso que motivó el arreglo: la carpeta sigue ahí, pero la canción que
    /// la biblioteca recuerda ya se borró. Antes se abría la carpeta y se
    /// contestaba que todo había salido bien.
    #[test]
    fn una_cancion_borrada_no_pasa_por_exito() {
        let folder = std::env::temp_dir().join("vasak-resonance-reveal-test");
        std::fs::create_dir_all(&folder).expect("no se pudo crear la carpeta de prueba");
        let missing = folder.join("cancion-que-ya-no-esta.mp3");
        let _ = std::fs::remove_file(&missing);

        assert_eq!(
            kind_of(missing.to_str().expect("ruta no representable")),
            RevealErrorKind::FileMissing
        );

        let _ = std::fs::remove_dir(&folder);
    }
}
