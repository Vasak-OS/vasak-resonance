import { computed } from 'vue';
import type { DroppedPlaybackTrack } from '@/services/player.service';

interface UseTrackTitleInput {
	currentTrack: () => DroppedPlaybackTrack | null;
	currentPath: () => string | null;
	fallback?: string;
}

export const useTrackTitle = ({
	currentTrack,
	currentPath,
	fallback = 'Sin reproduccion',
}: UseTrackTitleInput) => {
	return computed(() => {
		const track = currentTrack();
		const path = currentPath();

		if (track?.title) {
			return track.title;
		}

		if (path) {
			const normalized = path.replace(/\\/g, '/');
			const parts = normalized.split('/');
			return parts[parts.length - 1] || fallback;
		}

		return fallback;
	});
};
