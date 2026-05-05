import { invoke } from '@tauri-apps/api/core';

/**
 * Service for fetching and caching album cover images
 * Calls the Tauri backend to query free APIs (Deezer/MusicBrainz)
 * and cache images locally at ~/.cache/resonance/albums/
 */

const coverCache = new Map<string, string>();

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
export async function fetchAlbumCover(artist: string, album: string): Promise<string> {
	if (!artist || !album) {
		return '';
	}

	// With a cache hit we return immediately and do not invoke backend/API.
	const cacheKey = getCacheKey(artist, album);

	// Check in-memory cache first
	if (coverCache.has(cacheKey)) {
		return coverCache.get(cacheKey) || '';
	}

	try {
		const result = await invoke<string>('fetch_album_cover_command', {
			artist,
			album,
		});

		// Cache successful result
		coverCache.set(cacheKey, result);
		return result;
	} catch (error) {
		console.debug(
			`[album-cover] Failed to fetch cover for "${artist}" - "${album}": ${error instanceof Error ? error.message : String(error)}`
		);
		// Cache empty result to avoid repeated requests
		coverCache.set(cacheKey, '');
		return '';
	}
}

/**
 * Get cover URL with fallback strategy:
 * 1. Return cover_data_url if available (existing embedded cover)
 * 2. Try to fetch from cache/APIs if cover_data_url is empty
 */
export async function getOrFetchCoverUrl(
	artist: string,
	album: string,
	existingCoverDataUrl: string | null | undefined
): Promise<string> {
	// If we have an existing cover, use it
	if (existingCoverDataUrl) {
		return existingCoverDataUrl;
	}

	// Try to fetch from cache/APIs
	return fetchAlbumCover(artist, album);
}

/**
 * Clear in-memory cache (useful for testing or refresh)
 */
export function clearCoverCache(): void {
	coverCache.clear();
}

/**
 * Get cache stats for debugging
 */
export function getCoverCacheStats(): { size: number; keys: string[] } {
	return {
		size: coverCache.size,
		keys: Array.from(coverCache.keys()),
	};
}
