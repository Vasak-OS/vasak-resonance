import { invoke } from '@tauri-apps/api/core';

/**
 * Lo que la ventana sabe de Last.fm.
 *
 * `configurado` es si hay clave de API en esta instalación; sin eso la sección
 * de ajustes no se muestra, igual que con la presencia en Discord. `usuario` es
 * con quién está vinculado, si lo está.
 */
export interface EstadoDeLastfm {
	configurado: boolean;
	usuario: string | null;
}

const APAGADO: EstadoDeLastfm = { configurado: false, usuario: null };

/**
 * Ante cualquier error se contesta «apagado», que es el lado seguro: una
 * sección que no se dibuja es mucho mejor que una que promete vincular algo que
 * no se puede vincular.
 */
export async function estadoDeLastfm(): Promise<EstadoDeLastfm> {
	try {
		return await invoke<EstadoDeLastfm>('lastfm_status');
	} catch (error) {
		console.warn('[lastfm] No se pudo leer el estado:', error);
		return APAGADO;
	}
}

/**
 * Pide el token y abre el navegador en la página de autorización.
 *
 * El token que devuelve **todavía no sirve**: hay que guardarlo hasta que la
 * persona autorice allá y vuelva acá a decir que ya está.
 */
export async function empezarAutorizacion(): Promise<string> {
	return invoke<string>('lastfm_start_authorization');
}

/** Cambia el token ya autorizado por la sesión, que no vence. */
export async function terminarAutorizacion(token: string): Promise<EstadoDeLastfm> {
	return invoke<EstadoDeLastfm>('lastfm_finish_authorization', { token });
}

/** Deja de mandar escuchas y olvida la sesión. */
export async function desvincularLastfm(): Promise<EstadoDeLastfm> {
	return invoke<EstadoDeLastfm>('lastfm_disconnect');
}
