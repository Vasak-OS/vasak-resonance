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
	return invoke<void>('play_file', { file_path: filePath });
};

export const pausePlayback = (): Promise<void> => {
	return invoke<void>('pause');
};

export const resumePlayback = (): Promise<void> => {
	return invoke<void>('resume');
};

export const seekPlayback = (second: number): Promise<void> => {
	return invoke<void>('seek', { second: Math.max(0, Math.floor(second)) });
};

export const setPlaybackVolume = (volume: number): Promise<void> => {
	return invoke<void>('set_volume', { volume });
};

export const handleDroppedFile = (filePath: string): Promise<DroppedPlaybackTrack> => {
	return invoke<DroppedPlaybackTrack>('handle_dropped_file', { file_path: filePath });
};
