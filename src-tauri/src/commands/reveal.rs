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

#[tauri::command]
pub fn show_in_file_manager(path: String) -> Result<(), String> {
    let file = Path::new(&path);

    let folder = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| format!("La ruta no tiene carpeta contenedora: {path}"))?;

    if !folder.is_dir() {
        return Err(format!(
            "La carpeta de la canción ya no existe: {}",
            folder.display()
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
        .map_err(|error| format!("No se pudo abrir el gestor de archivos: {error}"))?;

    // `xdg-open` delega y termina enseguida. Sin nadie que lo espere quedaría un
    // proceso zombi por cada vez que se usa el menú, y el reproductor es una
    // ventana que se deja abierta todo el día.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}
