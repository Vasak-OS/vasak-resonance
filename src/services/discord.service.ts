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

/** Deja el perfil como estaba: al parar la música y al cerrar. */
export async function limpiarPresencia(): Promise<void> {
	try {
		await invoke('clear_discord_presence');
	} catch (error) {
		console.warn('[discord] No se pudo limpiar la presencia:', error);
	}
}
