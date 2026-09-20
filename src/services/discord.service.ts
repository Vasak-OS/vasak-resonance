import { invoke } from '@tauri-apps/api/core';
import type { EstadoParaDiscord } from '@/tools/discordPresence';

/**
 * El puente con Rust, que no espera a Discord: deja el mensaje en el hilo que
 * habla con el socket y vuelve.
 *
 * Un fallo acá no puede cortar la música, así que se registra y se sigue.
 */
export async function enviarPresencia(estado: EstadoParaDiscord): Promise<void> {
	try {
		await invoke('update_discord_presence', {
			title: estado.title,
			artist: estado.artist,
			albumArtUrl: estado.albumArtUrl,
			isPaused: estado.isPaused,
			durationSecs: Math.max(0, Math.round(estado.durationSeconds)),
			currentTimeSecs: Math.max(0, Math.round(estado.positionSeconds)),
		});
	} catch (error) {
		console.warn('[discord] No se pudo actualizar la presencia:', error);
	}
}

/**
 * Si la presencia está encendida.
 *
 * Sirve para no salir a buscar la tapa del álbum cuando nadie la va a mirar:
 * sin identificador de aplicación configurado el hilo de Discord ni siquiera
 * arranca. Ante cualquier error se responde que no, que es el lado seguro —no
 * se sale a la red por las dudas—.
 */
export async function presenciaActiva(): Promise<boolean> {
	try {
		return await invoke<boolean>('discord_presence_activa');
	} catch (error) {
		console.warn('[discord] No se pudo saber si la presencia está activa:', error);
		return false;
	}
}

/** Deja el perfil como estaba: al parar la música y al cerrar. */
export async function limpiarPresencia(): Promise<void> {
	try {
		await invoke('clear_discord_presence');
	} catch (error) {
		console.warn('[discord] No se pudo limpiar la presencia:', error);
	}
}
