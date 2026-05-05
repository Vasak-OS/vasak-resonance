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

const componentToHex = (value: number): string => {
	const clamped = Math.max(0, Math.min(255, Math.round(value)));
	return clamped.toString(16).padStart(2, '0').toUpperCase();
};

/**
 * Extract dominant color from a data URL image (covers from cache/API).
 */
export async function extractDominantColorFromDataUrl(dataUrl: string): Promise<string | null> {
	if (!dataUrl || !dataUrl.startsWith('data:image/')) {
		return null;
	}

	if (typeof window === 'undefined') {
		return null;
	}

	const image = await new Promise<HTMLImageElement>((resolve, reject) => {
		const img = new Image();
		img.onload = () => resolve(img);
		img.onerror = () => reject(new Error('Failed to decode cover image'));
		img.src = dataUrl;
	});

	const canvas = document.createElement('canvas');
	const context = canvas.getContext('2d');
	if (!context) {
		return null;
	}

	const sampleWidth = 48;
	const sampleHeight = 48;
	canvas.width = sampleWidth;
	canvas.height = sampleHeight;
	context.drawImage(image, 0, 0, sampleWidth, sampleHeight);

	const data = context.getImageData(0, 0, sampleWidth, sampleHeight).data;
	let red = 0;
	let green = 0;
	let blue = 0;
	let total = 0;

	for (let index = 0; index < data.length; index += 4) {
		const alpha = data[index + 3];
		if (alpha < 96) {
			continue;
		}

		red += data[index];
		green += data[index + 1];
		blue += data[index + 2];
		total += 1;
	}

	if (total === 0) {
		return null;
	}

	const dominant = `#${componentToHex(red / total)}${componentToHex(green / total)}${componentToHex(blue / total)}`;
	return dominant;
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
