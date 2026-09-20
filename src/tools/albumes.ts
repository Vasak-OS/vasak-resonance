/**
 * Cómo se juntan los temas en discos.
 *
 * Vive acá y no dentro de la vista porque son dos reglas que se equivocan
 * fácil y en silencio —el disco que se parte, el disco que se funde con otro—,
 * y así se las puede fijar con pruebas.
 */

/** Lo que hace falta saber de un tema para agruparlo. */
export interface TemaAgrupable {
	path: string;
	title: string;
	artist: string;
	album: string;
	album_artist?: string;
	track_no?: number;
	cover_data_url?: string | null;
}

/** Un tema dentro de su disco. */
export interface TemaDelDisco {
	path: string;
	title: string;
	artist: string;
	track_no: number;
}

/** Un disco, con sus temas en orden. */
export interface Disco {
	key: string;
	album: string;
	artist: string;
	cover: string;
	coverDataUrl: string | null;
	tracks: TemaDelDisco[];
	tracksPreview: TemaDelDisco[];
}

const ARTISTA_DESCONOCIDO = 'Unknown Artist';
const ALBUM_DESCONOCIDO = 'Unknown Album';

const normalizar = (valor: string) => valor.trim().toLowerCase();

/**
 * De quién es el disco.
 *
 * El artista del álbum cuando el archivo lo dice, y el del tema cuando no. Esa
 * es la etiqueta que existe para las recopilaciones: sin ella cada intérprete se
 * lleva su propio «álbum» y un disco de doce artistas queda partido en doce.
 */
export const artistaDelDisco = (tema: TemaAgrupable): string =>
	tema.album_artist || tema.artist || ARTISTA_DESCONOCIDO;

/**
 * La clave que identifica un disco.
 *
 * Lleva el artista **además** del nombre. Con el nombre solo, dos discos
 * distintos que se llamen igual —«Greatest Hits», «Live», «Demos», «Unplugged»,
 * que cualquier biblioteca mediana tiene repetidos— se funden en uno, y el
 * artista que termina mostrándose es el de la primera pista que haya caído ahí.
 */
export const claveDeDisco = (tema: TemaAgrupable): string =>
	`${normalizar(artistaDelDisco(tema))}|${normalizar(tema.album || ALBUM_DESCONOCIDO)}`;

/**
 * Ordena los temas como vienen en el disco.
 *
 * Los que no traen número quedan al final y entre ellos por título, que es lo
 * único que los ordena. Van al final y no al principio porque un archivo sin
 * numerar suele ser el agregado —la pista oculta, el bonus— y no la apertura.
 */
export const enOrdenDeDisco = (temas: TemaDelDisco[]): TemaDelDisco[] =>
	[...temas].sort((a, b) => {
		if (a.track_no !== b.track_no) {
			if (a.track_no === 0) return 1;
			if (b.track_no === 0) return -1;
			return a.track_no - b.track_no;
		}
		return a.title.localeCompare(b.title);
	});

/**
 * Junta los temas en discos, cada uno con sus pistas en orden.
 *
 * `tituloDesconocido` es el texto traducido para un tema sin título; se pasa
 * como argumento para que esto no dependa de la interfaz.
 */
export function agruparEnDiscos(temas: TemaAgrupable[], tituloDesconocido: string): Disco[] {
	const discos = new Map<string, Disco>();

	for (const tema of temas) {
		const key = claveDeDisco(tema);
		let disco = discos.get(key);

		if (!disco) {
			disco = {
				key,
				album: tema.album || ALBUM_DESCONOCIDO,
				artist: artistaDelDisco(tema),
				cover: tema.cover_data_url || '',
				coverDataUrl: tema.cover_data_url || null,
				tracks: [],
				tracksPreview: [],
			};
			discos.set(key, disco);
		}

		// La tapa la pone el primer tema que traiga una: en un disco bien
		// etiquetado son todas la misma, y en uno a medio etiquetar alcanza con
		// que una la tenga.
		if (!disco.cover && tema.cover_data_url) {
			disco.cover = tema.cover_data_url;
			disco.coverDataUrl = tema.cover_data_url;
		}

		disco.tracks.push({
			path: tema.path,
			title: tema.title || tituloDesconocido,
			artist: tema.artist || ARTISTA_DESCONOCIDO,
			track_no: tema.track_no ?? 0,
		});
	}

	return Array.from(discos.values())
		.map((disco) => {
			const tracks = enOrdenDeDisco(disco.tracks);
			return { ...disco, tracks, tracksPreview: tracks.slice(0, 4) };
		})
		.sort((a, b) => a.album.localeCompare(b.album));
}
