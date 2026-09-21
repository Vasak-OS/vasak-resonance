import { describe, expect, test } from 'bun:test';
import type { TemaAgrupable } from '../src/tools/albumes';
import { agruparPorArtista } from '../src/tools/artistas';

/**
 * La regla que distingue esta vista de la de álbumes: acá se agrupa por el
 * artista **del tema**. Una recopilación es «Varios artistas» como disco, pero
 * quien busca a alguien quiere encontrar también lo que hizo ahí.
 */

const tema = (parcial: Partial<TemaAgrupable> & { path: string }): TemaAgrupable => ({
	title: 'Un tema',
	artist: 'Alguien',
	album: 'Un álbum',
	...parcial,
});

const SIN_TITULO = 'Tema sin título';

describe('juntar los temas por artista', () => {
	test('cada artista con lo suyo', () => {
		const artistas = agruparPorArtista(
			[
				tema({ path: '/m/1.mp3', artist: 'Queen' }),
				tema({ path: '/m/2.mp3', artist: 'ABBA' }),
				tema({ path: '/m/3.mp3', artist: 'Queen' }),
			],
			SIN_TITULO
		);

		expect(artistas).toHaveLength(2);
		expect(artistas.map((a) => a.nombre)).toEqual(['ABBA', 'Queen']);
		expect(artistas.find((a) => a.nombre === 'Queen')?.temas).toHaveLength(2);
	});

	test('el intérprete de una recopilación aparece por su tema', () => {
		// Como disco, «Enganchados» es de «Varios». Pero el tema es de quien lo
		// cantó, y esta vista es la que tiene que encontrarlo.
		const artistas = agruparPorArtista(
			[
				tema({
					path: '/m/1.mp3',
					artist: 'Quien la canta',
					album: 'Enganchados',
					album_artist: 'Varios',
				}),
			],
			SIN_TITULO
		);

		expect(artistas).toHaveLength(1);
		expect(artistas[0].nombre).toBe('Quien la canta');
	});

	test('el mismo nombre en otra caja es el mismo artista', () => {
		const artistas = agruparPorArtista(
			[tema({ path: '/m/1.mp3', artist: 'Queen' }), tema({ path: '/m/2.mp3', artist: 'queen' })],
			SIN_TITULO
		);

		expect(artistas).toHaveLength(1);
		expect(artistas[0].temas).toHaveLength(2);
	});

	test('los discos de un artista se arman con las reglas de siempre', () => {
		// Dos discos suyos, y las pistas de cada uno en su orden.
		const artistas = agruparPorArtista(
			[
				tema({ path: '/m/2.mp3', album: 'Primero', title: 'Segunda', track_no: 2 }),
				tema({ path: '/m/1.mp3', album: 'Primero', title: 'Primera', track_no: 1 }),
				tema({ path: '/m/3.mp3', album: 'Segundo', title: 'Otra' }),
			],
			SIN_TITULO
		);

		expect(artistas[0].discos).toHaveLength(2);
		const primero = artistas[0].discos.find((d) => d.album === 'Primero');
		expect(primero?.tracks.map((t) => t.title)).toEqual(['Primera', 'Segunda']);
	});

	test('un tema sin artista cae en el centinela y no se pierde', () => {
		// «Unknown Artist» es lo que escribe el backend; la interfaz lo traduce
		// al mostrarlo.
		const artistas = agruparPorArtista([tema({ path: '/m/1.mp3', artist: '' })], SIN_TITULO);

		expect(artistas[0].nombre).toBe('Unknown Artist');
		expect(artistas[0].temas).toHaveLength(1);
	});

	test('la tapa sale del primer tema que traiga una', () => {
		const artistas = agruparPorArtista(
			[
				tema({ path: '/m/1.mp3', cover_data_url: null }),
				tema({ path: '/m/2.mp3', cover_data_url: 'data:image/png;base64,AAAA' }),
			],
			SIN_TITULO
		);

		expect(artistas[0].tapa).toBe('data:image/png;base64,AAAA');
	});

	test('los artistas salen ordenados por nombre', () => {
		const artistas = agruparPorArtista(
			[
				tema({ path: '/m/1.mp3', artist: 'Zeta' }),
				tema({ path: '/m/2.mp3', artist: 'Alfa' }),
				tema({ path: '/m/3.mp3', artist: 'Mu' }),
			],
			SIN_TITULO
		);

		expect(artistas.map((a) => a.nombre)).toEqual(['Alfa', 'Mu', 'Zeta']);
	});
});
