import { describe, expect, test } from 'bun:test';
import { type EstadoParaDiscord, hayQueAvisar } from '../src/tools/discordPresence';

/**
 * Discord dibuja la barra solo a partir de las marcas de tiempo, así que avisarle
 * en cada tic de reproducción sería mandarle por el socket algo que ya sabe.
 * Estos tests fijan cuándo sí hace falta.
 */
const sonando: EstadoParaDiscord = {
	path: '/musica/a.mp3',
	title: 'Bohemian Rhapsody',
	artist: 'Queen',
	albumArtUrl: null,
	isPaused: false,
	positionSeconds: 60,
	durationSeconds: 355,
};

describe('cuándo avisarle a Discord', () => {
	test('la primera vez siempre', () => {
		expect(hayQueAvisar(null, sonando, 0)).toBe(true);
	});

	test('reproducir normal no manda nada', () => {
		// Tres segundos después, la canción va por el segundo 63: exactamente
		// donde Discord ya la está mostrando.
		const actual = { ...sonando, positionSeconds: 63 };

		expect(hayQueAvisar(sonando, actual, 3)).toBe(false);
	});

	test('cambiar de canción sí', () => {
		const actual = { ...sonando, path: '/musica/b.mp3', positionSeconds: 0 };

		expect(hayQueAvisar(sonando, actual, 1)).toBe(true);
	});

	test('pausar y reanudar sí', () => {
		const enPausa = { ...sonando, isPaused: true };

		expect(hayQueAvisar(sonando, enPausa, 1)).toBe(true);
		expect(hayQueAvisar(enPausa, sonando, 1)).toBe(true);
	});

	test('cuando aparece la tapa sí', () => {
		// La tapa no está en el archivo: se busca aparte y llega unos segundos
		// después de que la canción empezó. Sin este aviso, Discord se quedaría
		// mostrando la tarjeta sin tapa hasta la canción siguiente.
		const conTapa = {
			...sonando,
			albumArtUrl: 'https://coverartarchive.org/release/abc/front',
		};

		expect(hayQueAvisar(sonando, conTapa, 1)).toBe(true);
	});

	test('que la tapa siga siendo la misma no manda nada', () => {
		const conTapa = {
			...sonando,
			albumArtUrl: 'https://coverartarchive.org/release/abc/front',
		};
		const tresSegundosDespues = { ...conTapa, positionSeconds: 63 };

		expect(hayQueAvisar(conTapa, tresSegundosDespues, 3)).toBe(false);
	});

	test('quedarse sin tapa también sí', () => {
		// Pasar de un álbum con tapa a uno sin ella: la tarjeta tiene que volver
		// al logo del sistema en vez de quedarse con la del álbum anterior.
		const conTapa = {
			...sonando,
			albumArtUrl: 'https://coverartarchive.org/release/abc/front',
		};

		expect(hayQueAvisar(conTapa, sonando, 1)).toBe(true);
	});

	test('saltar a otro punto sí', () => {
		const actual = { ...sonando, positionSeconds: 200 };

		expect(hayQueAvisar(sonando, actual, 1)).toBe(true);
	});

	test('en pausa el tiempo no corre', () => {
		// Diez minutos en pausa y la canción sigue donde estaba: no hay nada que
		// corregirle a Discord.
		const enPausa = { ...sonando, isPaused: true };
		const sigueEnPausa = { ...enPausa };

		expect(hayQueAvisar(enPausa, sigueEnPausa, 600)).toBe(false);
	});

	test('un tic que llega tarde no cuenta como salto', () => {
		// El tic dice 61 cuando pasaron 2 segundos: un segundo de desfase, que es
		// ruido normal y no un salto.
		const actual = { ...sonando, positionSeconds: 61 };

		expect(hayQueAvisar(sonando, actual, 2)).toBe(false);
	});
});
