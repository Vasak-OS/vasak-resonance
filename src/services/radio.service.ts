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

const CACHE_KEY = 'radio_stations_cache';

interface CachedStations {
	timestamp: number;
	stations: RadioStation[];
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

export function getCachedStations(): RadioStation[] | null {
	try {
		const cached = localStorage.getItem(CACHE_KEY);
		if (cached) {
			const data: CachedStations = JSON.parse(cached);
			// Cache valid for 1 hour
			if (Date.now() - data.timestamp < 3600000) {
				return data.stations;
			}
		}
	} catch (error) {
		console.error('Error reading station cache:', error);
	}
	return null;
}

export function setCachedStations(stations: RadioStation[]): void {
	try {
		const data: CachedStations = {
			timestamp: Date.now(),
			stations,
		};
		localStorage.setItem(CACHE_KEY, JSON.stringify(data));
	} catch (error) {
		console.error('Error saving station cache:', error);
	}
}
