import { describe, expect, test } from 'bun:test';
import { type GroupableTrack, groupIntoAlbums } from '../src/tools/albums';

/**
 * Dos reglas que se equivocan fácil y en silencio: el disco que se parte en uno
 * por intérprete, y el disco que se funde con otro que se llama igual. Las dos
 * pasaban antes de esto.
 */

const track = (partial: Partial<GroupableTrack> & { path: string }): GroupableTrack => ({
	title: 'Un tema',
	artist: 'Alguien',
	album: 'Un álbum',
	...partial,
});

const UNTITLED = 'Tema sin título';

describe('juntar los temas en discos', () => {
	test('una recopilación es un disco, no uno por intérprete', () => {
		// Cada pista tiene su intérprete y todas comparten el álbum: es
		// exactamente para esto que existe la etiqueta del artista del álbum.
		const albums = groupIntoAlbums(
			[
				track({
					path: '/m/1.mp3',
					artist: 'Primera',
					album: 'Enganchados',
					album_artist: 'Varios',
				}),
				track({
					path: '/m/2.mp3',
					artist: 'Segunda',
					album: 'Enganchados',
					album_artist: 'Varios',
				}),
				track({
					path: '/m/3.mp3',
					artist: 'Tercera',
					album: 'Enganchados',
					album_artist: 'Varios',
				}),
			],
			UNTITLED
		);

		expect(albums).toHaveLength(1);
		expect(albums[0].artist).toBe('Varios');
		expect(albums[0].tracks).toHaveLength(3);
	});

	test('dos discos que se llaman igual no se funden', () => {
		// «Greatest Hits» lo tiene repetido cualquier biblioteca mediana.
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/q.mp3', artist: 'Queen', album: 'Greatest Hits' }),
				track({ path: '/m/a.mp3', artist: 'ABBA', album: 'Greatest Hits' }),
			],
			UNTITLED
		);

		expect(albums).toHaveLength(2);
		expect(albums.map((album) => album.artist).sort()).toEqual(['ABBA', 'Queen']);
	});

	test('un separador en el nombre del artista no funde dos discos', () => {
		// Con la clave pegada con «|», el artista «A|B» con el álbum «C» armaba
		// la misma clave que el artista «A» con el álbum «B|C».
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/1.mp3', artist: 'A|B', album: 'C' }),
				track({ path: '/m/2.mp3', artist: 'A', album: 'B|C' }),
			],
			UNTITLED
		);

		expect(albums).toHaveLength(2);
	});

	test('el mismo disco con el nombre en otra caja sigue siendo uno', () => {
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/1.mp3', artist: 'Queen', album: 'A Night at the Opera' }),
				track({ path: '/m/2.mp3', artist: 'queen', album: 'a night at the opera' }),
			],
			UNTITLED
		);

		expect(albums).toHaveLength(1);
	});

	test('las pistas salen en el orden del disco', () => {
		// Llegan en cualquier orden: el de indexado es el del barrido del disco
		// duro.
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/c.mp3', title: 'Outro', track_no: 3 }),
				track({ path: '/m/a.mp3', title: 'Intro', track_no: 1 }),
				track({ path: '/m/b.mp3', title: 'La del medio', track_no: 2 }),
			],
			UNTITLED
		);

		expect(albums[0].tracks.map((albumTrack) => albumTrack.title)).toEqual([
			'Intro',
			'La del medio',
			'Outro',
		]);
	});

	test('las pistas sin numerar quedan al final, entre ellas por título', () => {
		// Un archivo sin numerar suele ser el agregado —la pista oculta, el
		// bonus—, no la apertura.
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/z.mp3', title: 'Zeta sin número' }),
				track({ path: '/m/a.mp3', title: 'Alfa sin número' }),
				track({ path: '/m/1.mp3', title: 'La primera', track_no: 1 }),
			],
			UNTITLED
		);

		expect(albums[0].tracks.map((albumTrack) => albumTrack.title)).toEqual([
			'La primera',
			'Alfa sin número',
			'Zeta sin número',
		]);
	});

	test('la tapa la pone el primer tema que traiga una', () => {
		// Un disco a medio etiquetar: alcanza con que una pista la tenga.
		const albums = groupIntoAlbums(
			[
				track({ path: '/m/1.mp3', track_no: 1, cover_data_url: null }),
				track({ path: '/m/2.mp3', track_no: 2, cover_data_url: 'data:image/png;base64,AAAA' }),
			],
			UNTITLED
		);

		expect(albums[0].cover).toBe('data:image/png;base64,AAAA');
	});

	test('un tema sin título muestra el texto que le pasan', () => {
		const albums = groupIntoAlbums([track({ path: '/m/1.mp3', title: '' })], UNTITLED);

		expect(albums[0].tracks[0].title).toBe(UNTITLED);
	});
});
