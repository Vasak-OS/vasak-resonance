import { computed } from 'vue';
import type { DroppedPlaybackTrack } from '@/services/player.service';

interface UseTrackSubtitleInput {
	currentTrack: () => DroppedPlaybackTrack | null;
	noTrackFallback?: string;
	unknownArtistLabel?: string;
	unknownAlbumLabel?: string;
}

export const useTrackSubtitle = ({
	currentTrack,
	noTrackFallback = 'Vasak Resonance',
	unknownArtistLabel = 'Unknown Artist',
	unknownAlbumLabel = 'Unknown Album',
}: UseTrackSubtitleInput) => {
	return computed(() => {
		const track = currentTrack();
		if (!track) {
			return noTrackFallback;
		}

		const artist = track.artist || unknownArtistLabel;
		const album = track.album || unknownAlbumLabel;
		return `${artist} • ${album}`;
	});
};
