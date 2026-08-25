import { computed } from 'vue';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import type { DroppedPlaybackTrack } from '@/services/player.service';

interface UseTrackSubtitleInput {
	currentTrack: () => DroppedPlaybackTrack | null;
	noTrackFallback?: string;
}

export const useTrackSubtitle = ({
	currentTrack,
	// El nombre de la aplicación: no se traduce.
	noTrackFallback = 'Vasak Resonance',
}: UseTrackSubtitleInput) => {
	const { artistLabel, albumLabel } = useMetadataLabels();

	return computed(() => {
		const track = currentTrack();
		if (!track) {
			return noTrackFallback;
		}

		return `${artistLabel(track.artist)} • ${albumLabel(track.album)}`;
	});
};
