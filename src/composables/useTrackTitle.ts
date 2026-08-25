import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import type { DroppedPlaybackTrack } from '@/services/player.service';

interface UseTrackTitleInput {
	currentTrack: () => DroppedPlaybackTrack | null;
	currentPath: () => string | null;
	fallback?: string;
}

export const useTrackTitle = ({ currentTrack, currentPath, fallback }: UseTrackTitleInput) => {
	const { t } = useI18n();

	return computed(() => {
		const track = currentTrack();
		const path = currentPath();
		const emptyLabel = fallback ?? t('player.noTrack');

		if (track?.title) {
			return track.title;
		}

		if (path) {
			const normalized = path.replace(/\\/g, '/');
			const parts = normalized.split('/');
			return parts[parts.length - 1] || emptyLabel;
		}

		return emptyLabel;
	});
};
