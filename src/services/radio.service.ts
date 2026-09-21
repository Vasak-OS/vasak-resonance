import { invoke } from '@tauri-apps/api/core';

export interface RadioStation {
	uuid: string;
	name: string;
	url: string;
	homepage?: string;
	favicon?: string;
	tags?: string;
	country?: string;
	state?: string;
	language?: string;
	votes?: number;
	codec?: string;
	bitrate?: number;
}

/**
 * Dónde se guardan las emisoras ya buscadas.
 *
 * **Una entrada por etiqueta.** Con una sola para todas, cambiar el filtro
 * mostraba las emisoras de la etiqueta anterior hasta que llegara la respuesta,
 * y después guardaba las nuevas encima: el caché nunca decía lo que decía.
 */
const CACHE_KEY_BASE = 'radio_stations_cache';

const cacheKeyFor = (tags: string[]): string => {
	const etiqueta = [...tags]
		.map((tag) => tag.trim().toLowerCase())
		.filter(Boolean)
		.sort()
		.join(',');
	return `${CACHE_KEY_BASE}:${etiqueta || 'music'}`;
};

interface CachedStations {
	timestamp: number;
	stations: RadioStation[];
}

/** Hasta cuándo se considera fresco lo guardado: una hora. */
const CACHE_FRESCO_MS = 3_600_000;

/** Lo guardado, con su antigüedad, sin decidir si sirve. */
function readCache(tags: string[]): CachedStations | null {
	try {
		const cached = localStorage.getItem(cacheKeyFor(tags));
		if (!cached) {
			return null;
		}
		const data: CachedStations = JSON.parse(cached);
		if (!Array.isArray(data.stations) || data.stations.length === 0) {
			return null;
		}
		return data;
	} catch (error) {
		console.error('Error reading station cache:', error);
		return null;
	}
}

export async function fetchRadioStations(tags: string[]): Promise<RadioStation[]> {
	try {
		const stations = await invoke<RadioStation[]>('fetch_radio_stations', { tags });
		return stations;
	} catch (error) {
		console.error('Error fetching radio stations:', error);
		throw error;
	}
}

export async function playRadioStation(station: RadioStation): Promise<void> {
	try {
		await invoke('play_radio_stream', {
			url: station.url,
			// Send both snake_case and camelCase to match Tauri's generated arg names
			station_name: station.name,
			stationName: station.name,
		});
	} catch (error) {
		console.error('Error playing radio station:', error);
		throw error;
	}
}

/** Lo guardado para esa etiqueta, si todavía está fresco. */
export function getCachedStations(tags: string[]): RadioStation[] | null {
	const guardado = readCache(tags);
	if (!guardado) {
		return null;
	}
	return Date.now() - guardado.timestamp < CACHE_FRESCO_MS ? guardado.stations : null;
}

/**
 * Lo guardado para esa etiqueta aunque esté viejo.
 *
 * Es lo que se muestra cuando el directorio no contesta. Antes esto también
 * caducaba a la hora, o sea que justo cuando más falta hacía —sin red, un rato
 * después— no quedaba nada que mostrar.
 */
export function getStaleCachedStations(tags: string[]): RadioStation[] | null {
	return readCache(tags)?.stations ?? null;
}

export function setCachedStations(tags: string[], stations: RadioStation[]): void {
	try {
		const data: CachedStations = {
			timestamp: Date.now(),
			stations,
		};
		localStorage.setItem(cacheKeyFor(tags), JSON.stringify(data));
	} catch (error) {
		console.error('Error saving station cache:', error);
	}
}
