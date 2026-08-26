import { invoke } from '@tauri-apps/api/core';

/**
 * Service for fetching and caching album cover images
 * Calls the Tauri backend to query free APIs (Deezer/MusicBrainz)
 * and cache images locally at ~/.cache/resonance/albums/
 */

const coverCache = new Map<string, PortadaConColor>();

/**
 * Generate cache key from artist and album
 */
function getCacheKey(artist: string, album: string): string {
	const normalizedArtist = artist.trim().toLowerCase();
	const normalizedAlbum = album.trim().toLowerCase();
	return `${normalizedArtist}|${normalizedAlbum}`;
}

/**
 * Fetch album cover from backend
 * Returns file:// URL if successful, empty string if not found
 */
/** Lo que devuelve el backend: la portada y el color que la representa. */
export interface PortadaConColor {
	cover_data_url: string;
	/** `#RRGGBB`, o cadena vacía si la imagen no se pudo leer. */
	dominant_color: string;
}

const SIN_PORTADA: PortadaConColor = { cover_data_url: '', dominant_color: '' };

export async function fetchAlbumCover(
	artist: string,
	album: string
): Promise<PortadaConColor> {
	if (!artist || !album) {
		return SIN_PORTADA;
	}

	// With a cache hit we return immediately and do not invoke backend/API.
	const cacheKey = getCacheKey(artist, album);

	// Check in-memory cache first
	const enCache = coverCache.get(cacheKey);
	if (enCache) {
		return enCache;
	}

	try {
		// El color viene con la portada: los bytes ya estaban del lado de Rust,
		// así que calcularlo allá le ahorra a esta ventana decodificar la imagen
		// otra vez para promediar píxeles a mano.
		const resultado = await invoke<PortadaConColor>('fetch_album_cover_command', {
			artist,
			album,
		});

		coverCache.set(cacheKey, resultado);
		return resultado;
	} catch (error) {
		console.debug(
			`[album-cover] Failed to fetch cover for "${artist}" - "${album}": ${error instanceof Error ? error.message : String(error)}`
		);
		// Se recuerda el fallo para no volver a pedirlo en cada redibujado.
		coverCache.set(cacheKey, SIN_PORTADA);
		return SIN_PORTADA;
	}
}


