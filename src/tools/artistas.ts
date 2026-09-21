/**
 * Cómo se juntan los temas por artista.
 *
 * Al lado de `albumes.ts` y por el mismo motivo: son reglas que se equivocan en
 * silencio, y acá se las puede fijar con pruebas.
 *
 * **Se agrupa por el artista del tema, no por el del álbum.** Es la diferencia
 * con los discos, y es a propósito: una recopilación es «Varios artistas» como
 * disco —eso está bien, el disco es de varios—, pero quien busca a alguien
 * quiere encontrar también el tema suelto que hizo ahí. Si esta vista agrupara
 * por el artista del álbum, ese tema no aparecería bajo su nombre en ningún
 * lado.
 */

import { agruparEnDiscos, type Disco, type TemaAgrupable } from '@/tools/albumes';

const ARTISTA_DESCONOCIDO = 'Unknown Artist';

const normalizar = (valor: string) => valor.trim().toLowerCase();

/** Un artista, con lo suyo. */
export interface Artista {
	clave: string;
	/** El nombre tal como viene en las etiquetas, para mostrarlo. */
	nombre: string;
	/** Sus temas, en el orden en que llegaron. */
	temas: TemaAgrupable[];
	/** Sus discos, armados con las mismas reglas que la vista de álbumes. */
	discos: Disco[];
	/** La primera tapa que aparezca entre sus temas, para la tarjeta. */
	tapa: string | null;
}

/** De quién es el tema. */
export const artistaDelTema = (tema: TemaAgrupable): string =>
	tema.artist?.trim() || ARTISTA_DESCONOCIDO;

/**
 * La clave que identifica a un artista.
 *
 * Sólo el nombre, normalizado: dos artistas distintos con el mismo nombre no se
 * pueden distinguir con lo que hay en las etiquetas, y fingir que sí —metiendo
 * el álbum en la clave, por ejemplo— partiría a cada artista en uno por disco.
 */
export const claveDeArtista = (tema: TemaAgrupable): string => normalizar(artistaDelTema(tema));

/**
 * Junta los temas por artista, cada uno con sus discos.
 *
 * `tituloDesconocido` es el texto traducido para un tema sin título; viaja hasta
 * acá porque los discos de cada artista se arman con `agruparEnDiscos`, que lo
 * necesita.
 */
export function agruparPorArtista(temas: TemaAgrupable[], tituloDesconocido: string): Artista[] {
	const artistas = new Map<string, Artista>();

	for (const tema of temas) {
		const clave = claveDeArtista(tema);
		let artista = artistas.get(clave);

		if (!artista) {
			artista = {
				clave,
				nombre: artistaDelTema(tema),
				temas: [],
				discos: [],
				tapa: null,
			};
			artistas.set(clave, artista);
		}

		artista.temas.push(tema);
		if (!artista.tapa && tema.cover_data_url) {
			artista.tapa = tema.cover_data_url;
		}
	}

	return Array.from(artistas.values())
		.map((artista) => ({
			...artista,
			discos: agruparEnDiscos(artista.temas, tituloDesconocido),
		}))
		.sort((a, b) => a.nombre.localeCompare(b.nombre));
}
