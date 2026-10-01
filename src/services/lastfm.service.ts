import { invoke } from '@tauri-apps/api/core';

/**
 * Lo que la ventana sabe de Last.fm.
 *
 * `configured` es si hay clave de API en esta instalación; sin eso la sección
 * de ajustes no se muestra, igual que con la presencia en Discord. `user` es
 * con quién está vinculado, si lo está.
 */
export interface LastfmStatus {
	configured: boolean;
	user: string | null;
}

/**
 * Lo que contesta Rust, con los nombres de los campos de `lastfm.rs` tal como
 * los serializa. Se traduce acá, en el borde, y una sola vez: el resto de la
 * ventana habla de `LastfmStatus`.
 */
interface LastfmStatusWire {
	configurado?: boolean;
	usuario?: string | null;
}

const OFF: LastfmStatus = { configured: false, user: null };

function fromWire(wire: LastfmStatusWire | null | undefined): LastfmStatus {
	return { configured: Boolean(wire?.configurado), user: wire?.usuario ?? null };
}

/**
 * Ante cualquier error se contesta «apagado», que es el lado seguro: una
 * sección que no se dibuja es mucho mejor que una que promete vincular algo que
 * no se puede vincular.
 */
export async function getLastfmStatus(): Promise<LastfmStatus> {
	try {
		return fromWire(await invoke<LastfmStatusWire>('lastfm_status'));
	} catch (error) {
		console.warn('[lastfm] No se pudo leer el estado:', error);
		return OFF;
	}
}

/**
 * Pide el token y abre el navegador en la página de autorización.
 *
 * El token que devuelve **todavía no sirve**: hay que guardarlo hasta que la
 * persona autorice allá y vuelva acá a decir que ya está.
 */
export async function startLastfmAuthorization(): Promise<string> {
	return invoke<string>('lastfm_start_authorization');
}

/** Cambia el token ya autorizado por la sesión, que no vence. */
export async function finishLastfmAuthorization(token: string): Promise<LastfmStatus> {
	return fromWire(await invoke<LastfmStatusWire>('lastfm_finish_authorization', { token }));
}

/** Deja de mandar escuchas y olvida la sesión. */
export async function unlinkLastfm(): Promise<LastfmStatus> {
	return fromWire(await invoke<LastfmStatusWire>('lastfm_disconnect'));
}
