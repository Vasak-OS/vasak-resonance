import { invoke } from '@tauri-apps/api/core';
import { devLog } from '@/composables/useDevLog';

export interface Playlist {
	id: number;
	name: string;
	created_at: string;
}

export interface PlaylistTrack {
	playlist_id: number;
	track_id: number;
	position: number;
	path: string;
	title: string;
	artist: string;
	album: string;
	duration_seconds: number;
}

export const listPlaylists = (): Promise<Playlist[]> => {
	devLog('[playlists.service] listPlaylists invoke');
	return invoke<Playlist[]>('list_playlists_command');
};

export const createPlaylist = (name: string): Promise<Playlist> => {
	devLog('[playlists.service] createPlaylist invoke:', name);
	return invoke<Playlist>('create_playlist_command', { name });
};

export const deletePlaylist = (playlistId: number): Promise<void> => {
	devLog('[playlists.service] deletePlaylist invoke:', playlistId);
	return invoke<void>('delete_playlist_command', { playlistId });
};

export const listPlaylistTracks = (playlistId: number): Promise<PlaylistTrack[]> => {
	devLog('[playlists.service] listPlaylistTracks invoke:', playlistId);
	return invoke<PlaylistTrack[]>('list_playlist_tracks_command', { playlistId });
};

export const addTrackToPlaylist = (playlistId: number, trackId: number): Promise<void> => {
	devLog('[playlists.service] addTrackToPlaylist invoke:', playlistId, trackId);
	return invoke<void>('add_track_to_playlist_command', { playlistId, trackId });
};

export const removeTrackFromPlaylist = (playlistId: number, trackId: number): Promise<void> => {
	devLog('[playlists.service] removeTrackFromPlaylist invoke:', playlistId, trackId);
	return invoke<void>('remove_track_from_playlist_command', { playlistId, trackId });
};
