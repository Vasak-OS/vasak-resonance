import { describe, expect, test } from 'bun:test';
import type { GroupableTrack } from '../src/tools/albums';
import { groupByArtist } from '../src/tools/artists';

/**
 * La regla que distingue esta vista de la de álbumes: acá se agrupa por el
 * artista **del tema**. Una recopilación es «Varios artistas» como disco, pero
 * quien busca a alguien quiere encontrar también lo que hizo ahí.
 */

const track = (partial: Partial<GroupableTrack> & { path: string }): GroupableTrack => ({
	title: 'Un tema',
	artist: 'Alguien',
	album: 'Un álbum',
	...partial,
});

const UNTITLED = 'Tema sin título';

describe('juntar los temas por artista', () => {
	test('cada artista con lo suyo', () => {
		const artists = groupByArtist(
			[
				track({ path: '/m/1.mp3', artist: 'Queen' }),
				track({ path: '/m/2.mp3', artist: 'ABBA' }),
				track({ path: '/m/3.mp3', artist: 'Queen' }),
			],
			UNTITLED
		);

		expect(artists).toHaveLength(2);
		expect(artists.map((a) => a.name)).toEqual(['ABBA', 'Queen']);
		expect(artists.find((a) => a.name === 'Queen')?.tracks).toHaveLength(2);
	});

	test('el intérprete de una recopilación aparece por su tema', () => {
		// Como disco, «Enganchados» es de «Varios». Pero el tema es de quien lo
		// cantó, y esta vista es la que tiene que encontrarlo.
		const artists = groupByArtist(
			[
				track({
					path: '/m/1.mp3',
					artist: 'Quien la canta',
					album: 'Enganchados',
					album_artist: 'Varios',
				}),
			],
			UNTITLED
		);

		expect(artists).toHaveLength(1);
		expect(artists[0].name).toBe('Quien la canta');
	});

	test('el mismo nombre en otra caja es el mismo artista', () => {
		const artists = groupByArtist(
			[track({ path: '/m/1.mp3', artist: 'Queen' }), track({ path: '/m/2.mp3', artist: 'queen' })],
			UNTITLED
		);

		expect(artists).toHaveLength(1);
		expect(artists[0].tracks).toHaveLength(2);
	});

	test('los discos de un artista se arman con las reglas de siempre', () => {
		// Dos discos suyos, y las pistas de cada uno en su orden.
		const artists = groupByArtist(
			[
				track({ path: '/m/2.mp3', album: 'Primero', title: 'Segunda', track_no: 2 }),
				track({ path: '/m/1.mp3', album: 'Primero', title: 'Primera', track_no: 1 }),
				track({ path: '/m/3.mp3', album: 'Segundo', title: 'Otra' }),
			],
			UNTITLED
		);

		expect(artists[0].albums).toHaveLength(2);
		const first = artists[0].albums.find((d) => d.album === 'Primero');
		expect(first?.tracks.map((t) => t.title)).toEqual(['Primera', 'Segunda']);
	});

	test('un tema sin artista cae en el centinela y no se pierde', () => {
		// «Unknown Artist» es lo que escribe el backend; la interfaz lo traduce
		// al mostrarlo.
		const artists = groupByArtist([track({ path: '/m/1.mp3', artist: '' })], UNTITLED);

		expect(artists[0].name).toBe('Unknown Artist');
		expect(artists[0].tracks).toHaveLength(1);
	});

	test('la tapa sale del primer tema que traiga una', () => {
		const artists = groupByArtist(
			[
				track({ path: '/m/1.mp3', cover_data_url: null }),
				track({ path: '/m/2.mp3', cover_data_url: 'data:image/png;base64,AAAA' }),
			],
			UNTITLED
		);

		expect(artists[0].cover).toBe('data:image/png;base64,AAAA');
	});

	test('los artistas salen ordenados por nombre', () => {
		const artists = groupByArtist(
			[
				track({ path: '/m/1.mp3', artist: 'Zeta' }),
				track({ path: '/m/2.mp3', artist: 'Alfa' }),
				track({ path: '/m/3.mp3', artist: 'Mu' }),
			],
			UNTITLED
		);

		expect(artists.map((a) => a.name)).toEqual(['Alfa', 'Mu', 'Zeta']);
	});
});
