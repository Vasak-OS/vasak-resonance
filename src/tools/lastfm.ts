import type { LastfmStatus } from '@/services/lastfm.service';

/**
 * En qué punto está la vinculación con Last.fm.
 *
 * - `hidden`: no hay clave de API en esta instalación, así que la sección no se
 *   dibuja. Es el caso de casi todo el mundo y no es un error.
 * - `unlinked`: hay clave, pero nadie autorizó todavía.
 * - `authorizing`: se pidió un token y se abrió el navegador; falta que la
 *   persona vuelva y confirme.
 * - `linked`: hay sesión guardada.
 */
export type LastfmStep = 'hidden' | 'unlinked' | 'authorizing' | 'linked';

/**
 * El paso, que sale del estado y no de una variable aparte.
 *
 * El orden importa: estar vinculado gana sobre tener un token a medio usar. Si
 * no, confirmar la autorización dejaría la sección pidiendo confirmar para
 * siempre —el token pendiente sigue ahí— aunque la cuenta ya esté vinculada.
 */
export function lastfmStep(status: LastfmStatus, pendingToken: string | null): LastfmStep {
	if (!status.configured) {
		return 'hidden';
	}

	if (status.user) {
		return 'linked';
	}

	return pendingToken ? 'authorizing' : 'unlinked';
}
