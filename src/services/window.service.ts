import { invoke } from '@tauri-apps/api/core';

export const toggleMainAndMiniPlayer = (): Promise<void> => {
	return invoke<void>('toggle_main_and_miniplayer');
};
