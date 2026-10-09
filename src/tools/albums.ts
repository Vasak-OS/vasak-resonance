/**
 * Cómo se juntan los temas en discos.
 *
 * Vive acá y no dentro de la vista porque son dos reglas que se equivocan
 * fácil y en silencio —el disco que se parte, el disco que se funde con otro—,
 * y así se las puede fijar con pruebas.
 */

/** Lo que hace falta saber de un tema para agruparlo. */
export interface GroupableTrack {
	path: string;
	title: string;
	artist: string;
	album: string;
	album_artist?: string;
	track_no?: number;
	cover_data_url?: string | null;
}

/** Un tema dentro de su disco. */
export interface AlbumTrack {
	path: string;
	title: string;
	artist: string;
	track_no: number;
}

/** Un disco, con sus temas en orden. */
export interface Album {
	key: string;
	album: string;
	artist: string;
	cover: string;
	coverDataUrl: string | null;
	tracks: AlbumTrack[];
	tracksPreview: AlbumTrack[];
}

const UNKNOWN_ARTIST = 'Unknown Artist';
const UNKNOWN_ALBUM = 'Unknown Album';

const normalize = (value: string) => value.trim().toLowerCase();

/**
 * De quién es el disco.
 *
 * El artista del álbum cuando el archivo lo dice, y el del tema cuando no. Esa
 * es la etiqueta que existe para las recopilaciones: sin ella cada intérprete se
 * lleva su propio «álbum» y un disco de doce artistas queda partido en doce.
 */
export const albumArtistOf = (track: GroupableTrack): string =>
	track.album_artist || track.artist || UNKNOWN_ARTIST;

/**
 * La clave que identifica un disco.
 *
 * Lleva el artista **además** del nombre. Con el nombre solo, dos discos
 * distintos que se llamen igual —«Greatest Hits», «Live», «Demos», «Unplugged»,
 * que cualquier biblioteca mediana tiene repetidos— se funden en uno, y el
 * artista que termina mostrándose es el de la primera pista que haya caído ahí.
 *
 * Los dos van serializados y no pegados con un separador: con un separador
 * suelto, un artista que lo tenga en el nombre arma la misma clave que otro
 * disco —`A|B` con el álbum `C` da lo mismo que `A` con el álbum `B|C`— y
 * vuelve a pasar lo que esto viene a arreglar.
 */
export const albumKeyOf = (track: GroupableTrack): string =>
	JSON.stringify([normalize(albumArtistOf(track)), normalize(track.album || UNKNOWN_ALBUM)]);

/**
 * Ordena los temas como vienen en el disco.
 *
 * Los que no traen número quedan al final y entre ellos por título, que es lo
 * único que los ordena. Van al final y no al principio porque un archivo sin
 * numerar suele ser el agregado —la pista oculta, el bonus— y no la apertura.
 */
export const inAlbumOrder = (tracks: AlbumTrack[]): AlbumTrack[] =>
	[...tracks].sort((a, b) => {
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
 * `unknownTitle` es el texto traducido para un tema sin título; se pasa
 * como argumento para que esto no dependa de la interfaz.
 */
export function groupIntoAlbums(tracks: GroupableTrack[], unknownTitle: string): Album[] {
	const albums = new Map<string, Album>();

	for (const track of tracks) {
		const key = albumKeyOf(track);
		let album = albums.get(key);

		if (!album) {
			album = {
				key,
				album: track.album || UNKNOWN_ALBUM,
				artist: albumArtistOf(track),
				cover: track.cover_data_url || '',
				coverDataUrl: track.cover_data_url || null,
				tracks: [],
				tracksPreview: [],
			};
			albums.set(key, album);
		}

		// La tapa la pone el primer tema que traiga una: en un disco bien
		// etiquetado son todas la misma, y en uno a medio etiquetar alcanza con
		// que una la tenga.
		if (!album.cover && track.cover_data_url) {
			album.cover = track.cover_data_url;
			album.coverDataUrl = track.cover_data_url;
		}

		album.tracks.push({
			path: track.path,
			title: track.title || unknownTitle,
			artist: track.artist || UNKNOWN_ARTIST,
			track_no: track.track_no ?? 0,
		});
	}

	return Array.from(albums.values())
		.map((album) => {
			const tracks = inAlbumOrder(album.tracks);
			return { ...album, tracks, tracksPreview: tracks.slice(0, 4) };
		})
		.sort((a, b) => a.album.localeCompare(b.album));
}
