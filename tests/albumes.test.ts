import { describe, expect, test } from 'bun:test';
import { agruparEnDiscos, type TemaAgrupable } from '../src/tools/albumes';

/**
 * Dos reglas que se equivocan fácil y en silencio: el disco que se parte en uno
 * por intérprete, y el disco que se funde con otro que se llama igual. Las dos
 * pasaban antes de esto.
 */

const tema = (parcial: Partial<TemaAgrupable> & { path: string }): TemaAgrupable => ({
	title: 'Un tema',
	artist: 'Alguien',
	album: 'Un álbum',
	...parcial,
});

const SIN_TITULO = 'Tema sin título';

describe('juntar los temas en discos', () => {
	test('una recopilación es un disco, no uno por intérprete', () => {
		// Cada pista tiene su intérprete y todas comparten el álbum: es
		// exactamente para esto que existe la etiqueta del artista del álbum.
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/1.mp3', artist: 'Primera', album: 'Enganchados', album_artist: 'Varios' }),
				tema({ path: '/m/2.mp3', artist: 'Segunda', album: 'Enganchados', album_artist: 'Varios' }),
				tema({ path: '/m/3.mp3', artist: 'Tercera', album: 'Enganchados', album_artist: 'Varios' }),
			],
			SIN_TITULO
		);

		expect(discos).toHaveLength(1);
		expect(discos[0].artist).toBe('Varios');
		expect(discos[0].tracks).toHaveLength(3);
	});

	test('dos discos que se llaman igual no se funden', () => {
		// «Greatest Hits» lo tiene repetido cualquier biblioteca mediana.
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/q.mp3', artist: 'Queen', album: 'Greatest Hits' }),
				tema({ path: '/m/a.mp3', artist: 'ABBA', album: 'Greatest Hits' }),
			],
			SIN_TITULO
		);

		expect(discos).toHaveLength(2);
		expect(discos.map((disco) => disco.artist).sort()).toEqual(['ABBA', 'Queen']);
	});

	test('el mismo disco con el nombre en otra caja sigue siendo uno', () => {
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/1.mp3', artist: 'Queen', album: 'A Night at the Opera' }),
				tema({ path: '/m/2.mp3', artist: 'queen', album: 'a night at the opera' }),
			],
			SIN_TITULO
		);

		expect(discos).toHaveLength(1);
	});

	test('las pistas salen en el orden del disco', () => {
		// Llegan en cualquier orden: el de indexado es el del barrido del disco
		// duro.
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/c.mp3', title: 'Outro', track_no: 3 }),
				tema({ path: '/m/a.mp3', title: 'Intro', track_no: 1 }),
				tema({ path: '/m/b.mp3', title: 'La del medio', track_no: 2 }),
			],
			SIN_TITULO
		);

		expect(discos[0].tracks.map((pista) => pista.title)).toEqual([
			'Intro',
			'La del medio',
			'Outro',
		]);
	});

	test('las pistas sin numerar quedan al final, entre ellas por título', () => {
		// Un archivo sin numerar suele ser el agregado —la pista oculta, el
		// bonus—, no la apertura.
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/z.mp3', title: 'Zeta sin número' }),
				tema({ path: '/m/a.mp3', title: 'Alfa sin número' }),
				tema({ path: '/m/1.mp3', title: 'La primera', track_no: 1 }),
			],
			SIN_TITULO
		);

		expect(discos[0].tracks.map((pista) => pista.title)).toEqual([
			'La primera',
			'Alfa sin número',
			'Zeta sin número',
		]);
	});

	test('la tapa la pone el primer tema que traiga una', () => {
		// Un disco a medio etiquetar: alcanza con que una pista la tenga.
		const discos = agruparEnDiscos(
			[
				tema({ path: '/m/1.mp3', track_no: 1, cover_data_url: null }),
				tema({ path: '/m/2.mp3', track_no: 2, cover_data_url: 'data:image/png;base64,AAAA' }),
			],
			SIN_TITULO
		);

		expect(discos[0].cover).toBe('data:image/png;base64,AAAA');
	});

	test('un tema sin título muestra el texto que le pasan', () => {
		const discos = agruparEnDiscos([tema({ path: '/m/1.mp3', title: '' })], SIN_TITULO);

		expect(discos[0].tracks[0].title).toBe(SIN_TITULO);
	});
});
