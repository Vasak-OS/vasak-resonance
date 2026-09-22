import type { EstadoDeLastfm } from '@/services/lastfm.service';

/**
 * En qué punto está la vinculación con Last.fm.
 *
 * - `oculto`: no hay clave de API en esta instalación, así que la sección no se
 *   dibuja. Es el caso de casi todo el mundo y no es un error.
 * - `sin-vincular`: hay clave, pero nadie autorizó todavía.
 * - `autorizando`: se pidió un token y se abrió el navegador; falta que la
 *   persona vuelva y confirme.
 * - `vinculado`: hay sesión guardada.
 */
export type PasoDeLastfm = 'oculto' | 'sin-vincular' | 'autorizando' | 'vinculado';

/**
 * El paso, que sale del estado y no de una variable aparte.
 *
 * El orden importa: estar vinculado gana sobre tener un token a medio usar. Si
 * no, confirmar la autorización dejaría la sección pidiendo confirmar para
 * siempre —el token pendiente sigue ahí— aunque la cuenta ya esté vinculada.
 */
export function pasoDeLastfm(estado: EstadoDeLastfm, tokenPendiente: string | null): PasoDeLastfm {
	if (!estado.configurado) {
		return 'oculto';
	}

	if (estado.usuario) {
		return 'vinculado';
	}

	return tokenPendiente ? 'autorizando' : 'sin-vincular';
}
