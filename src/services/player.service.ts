import { invoke } from '@tauri-apps/api/core';

export interface DroppedPlaybackTrack {
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
	cover_data_url: string | null;
	dominant_color: string | null;
}

export interface NowPlayingMetadata {
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
	cover_data_url: string | null;
	dominant_color: string | null;
}

export interface PlaybackProgressEvent {
	path: string | null;
	position_seconds: number;
	duration_seconds: number | null;
	is_playing: boolean;
	is_paused: boolean;
	volume: number;
	now_playing: NowPlayingMetadata | null;
}

export interface LibraryTrack {
	id: number;
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
	created_at: string;
}

export const playFile = (filePath: string): Promise<void> => {
	console.log('[player.service] playFile invoke:', filePath);
	return invoke<void>('play_file', { filePath, file_path: filePath });
};

export const pausePlayback = (): Promise<void> => {
	console.log('[player.service] pausePlayback invoke');
	return invoke<void>('pause');
};

export const resumePlayback = (): Promise<void> => {
	console.log('[player.service] resumePlayback invoke');
	return invoke<void>('resume');
};

export const seekPlayback = (second: number): Promise<void> => {
	console.log('[player.service] seekPlayback invoke:', second);
	return invoke<void>('seek', { second: Math.max(0, Math.floor(second)) });
};

export const setPlaybackVolume = (volume: number): Promise<void> => {
	console.log('[player.service] setPlaybackVolume invoke:', volume);
	return invoke<void>('set_volume', { volume });
};

export const getPlaybackSnapshot = (): Promise<PlaybackProgressEvent> => {
	console.log('[player.service] getPlaybackSnapshot invoke');
	return invoke<PlaybackProgressEvent>('get_playback_snapshot');
};

export const stopPlayback = (): Promise<void> => {
	console.log('[player.service] stopPlayback invoke');
	return invoke<void>('stop');
};

export const handleDroppedFile = (filePath: string): Promise<DroppedPlaybackTrack> => {
	console.log('[player.service] handleDroppedFile invoke:', filePath);
	return invoke<DroppedPlaybackTrack>('handle_dropped_file', {
		filePath,
		file_path: filePath,
	});
};

export const listLibraryTracks = (): Promise<LibraryTrack[]> => {
	console.log('[player.service] listLibraryTracks invoke');
	return invoke<LibraryTrack[]>('list_library_tracks');
};

export const searchLibraryTracks = (query: string, limit?: number): Promise<LibraryTrack[]> => {
	console.log('[player.service] searchLibraryTracks invoke:', query);
	return invoke<LibraryTrack[]>('search_library_tracks', { query, limit });
};

export const saveLibraryTrack = (track: DroppedPlaybackTrack): Promise<void> => {
	console.log('[player.service] saveLibraryTrack invoke:', track.path);
	return invoke<void>('save_library_track', { track });
};
