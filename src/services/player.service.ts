import { invoke } from '@tauri-apps/api/core';

export interface DroppedPlaybackTrack {
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
	cover_data_url: string | null;
}

export interface NowPlayingMetadata {
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
	cover_data_url: string | null;
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
