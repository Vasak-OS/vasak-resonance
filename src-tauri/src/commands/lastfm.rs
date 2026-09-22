//! Vincular —y desvincular— una cuenta de Last.fm desde la ventana.
//!
//! Son tres llamadas y las tres van a la red, así que todas trabajan en un hilo
//! aparte: bloquear el hilo de la interfaz por diez segundos de espera dejaría
//! la ventana congelada mientras la red no contesta.

use std::process::{Command, Stdio};

use crate::lastfm::{self, Estado};

/// Si hay clave de API configurada y con quién está vinculado.
///
/// La ventana usa esto para decidir si **mostrar o no** la sección: sin clave no
/// se ve nada, igual que con la presencia en Discord.
#[tauri::command]
pub async fn lastfm_status() -> Estado {
    en_otro_hilo(lastfm::estado).await.unwrap_or(Estado {
        configurado: false,
        usuario: None,
    })
}

/// Empieza la autorización: pide un token y abre el navegador.
///
/// Devuelve el token, que la ventana tiene que guardar hasta que la persona
/// vuelva y diga que ya autorizó. Todavía no sirve para nada: hasta que alguien
/// apruebe en esa página, cambiarlo por una sesión falla.
#[tauri::command]
pub async fn lastfm_start_authorization() -> Result<String, String> {
    let (token, direccion) = en_otro_hilo(lastfm::pedir_autorizacion).await??;

    abrir_en_el_navegador(&direccion)?;

    Ok(token)
}

/// Cambia el token ya aprobado por la sesión, que no vence, y la guarda.
#[tauri::command]
pub async fn lastfm_finish_authorization(token: String) -> Result<Estado, String> {
    en_otro_hilo(move || lastfm::confirmar(&token)).await?
}

/// Deja de mandar escuchas y olvida la sesión.
#[tauri::command]
pub async fn lastfm_disconnect() -> Result<Estado, String> {
    en_otro_hilo(lastfm::desvincular).await?
}

/// Corre algo que bloquea, fuera del hilo de la interfaz.
async fn en_otro_hilo<T, F>(trabajo: F) -> Result<T, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(trabajo)
        .await
        .map_err(|error| format!("No se pudo hacer el trabajo en otro hilo: {error}"))
}

/// Abre la página de autorización.
///
/// Por `xdg-open` y no por el comando `open` del plugin de shell, que es la
/// misma decisión que tomó [`crate::commands::reveal`]: llamado desde la página
/// exige abrir el alcance, y acá la dirección la arma esta misma aplicación con
/// su clave y su token. No pasa por ningún intérprete de comandos: es un
/// argumento y nada más.
fn abrir_en_el_navegador(direccion: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(direccion)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("No se pudo abrir el navegador: {error}"))
}
