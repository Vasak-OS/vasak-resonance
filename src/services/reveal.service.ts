import { invoke } from '@tauri-apps/api/core';

/**
 * Abre en el gestor de archivos la carpeta que contiene la canción.
 *
 * Quien elige la carpeta y la abre es el backend: acá sólo se le pasa la ruta
 * del archivo tal como la tiene la biblioteca.
 */
export async function showInFileManager(path: string): Promise<void> {
	await invoke('show_in_file_manager', { path });
}
