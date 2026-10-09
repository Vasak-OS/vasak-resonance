/**
 * Cómo se juntan los temas por artista.
 *
 * Al lado de `albums.ts` y por el mismo motivo: son reglas que se equivocan en
 * silencio, y acá se las puede fijar con pruebas.
 *
 * **Se agrupa por el artista del tema, no por el del álbum.** Es la diferencia
 * con los discos, y es a propósito: una recopilación es «Varios artistas» como
 * disco —eso está bien, el disco es de varios—, pero quien busca a alguien
 * quiere encontrar también el tema suelto que hizo ahí. Si esta vista agrupara
 * por el artista del álbum, ese tema no aparecería bajo su nombre en ningún
 * lado.
 */

import { type Album, type GroupableTrack, groupIntoAlbums } from '@/tools/albums';

const UNKNOWN_ARTIST = 'Unknown Artist';

const normalize = (value: string) => value.trim().toLowerCase();

/** Un artista, con lo suyo. */
export interface Artist {
	key: string;
	/** El nombre tal como viene en las etiquetas, para mostrarlo. */
	name: string;
	/** Sus temas, en el orden en que llegaron. */
	tracks: GroupableTrack[];
	/** Sus discos, armados con las mismas reglas que la vista de álbumes. */
	albums: Album[];
	/** La primera tapa que aparezca entre sus temas, para la tarjeta. */
	cover: string | null;
}

/** De quién es el tema. */
export const trackArtistOf = (track: GroupableTrack): string =>
	track.artist?.trim() || UNKNOWN_ARTIST;

/**
 * La clave que identifica a un artista.
 *
 * Sólo el nombre, normalizado: dos artistas distintos con el mismo nombre no se
 * pueden distinguir con lo que hay en las etiquetas, y fingir que sí —metiendo
 * el álbum en la clave, por ejemplo— partiría a cada artista en uno por disco.
 */
export const artistKeyOf = (track: GroupableTrack): string => normalize(trackArtistOf(track));

/**
 * Junta los temas por artista, cada uno con sus discos.
 *
 * `unknownTitle` es el texto traducido para un tema sin título; viaja hasta
 * acá porque los discos de cada artista se arman con `groupIntoAlbums`, que lo
 * necesita.
 */
export function groupByArtist(tracks: GroupableTrack[], unknownTitle: string): Artist[] {
	const artists = new Map<string, Artist>();

	for (const track of tracks) {
		const key = artistKeyOf(track);
		let artist = artists.get(key);

		if (!artist) {
			artist = {
				key,
				name: trackArtistOf(track),
				tracks: [],
				albums: [],
				cover: null,
			};
			artists.set(key, artist);
		}

		artist.tracks.push(track);
		if (!artist.cover && track.cover_data_url) {
			artist.cover = track.cover_data_url;
		}
	}

	return Array.from(artists.values())
		.map((artist) => ({
			...artist,
			albums: groupIntoAlbums(artist.tracks, unknownTitle),
		}))
		.sort((a, b) => a.name.localeCompare(b.name));
}
