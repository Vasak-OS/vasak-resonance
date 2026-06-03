<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import PlayerBackground from '@/components/player/PlayerBackground.vue';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MiniTransportControls from '@/components/player/transport/MiniTransportControls.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { extractDominantColorFromDataUrl, fetchAlbumCover } from '@/services/album-cover.service';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
const fetchedCoverUrl = ref<string>('');

useConfigSync();

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});

const trackSubtitle = useTrackSubtitle({
	currentTrack: () => playerStore.currentTrack,
});

const coverSrc = computed(() => {
	// First try embedded cover
	if (playerStore.currentTrack?.cover_data_url) {
		return playerStore.currentTrack.cover_data_url;
	}
	// Fall back to fetched cover from cache/APIs
	return fetchedCoverUrl.value;
});

// Watch for track changes and fetch cover if needed
watch(
	() => playerStore.currentTrack?.path,
	async (newPath) => {
		if (!newPath) {
			fetchedCoverUrl.value = '';
			return;
		}

		const track = playerStore.currentTrack;
		if (!track) {
			fetchedCoverUrl.value = '';
			return;
		}

		// If track has embedded cover, don't fetch
		if (track.cover_data_url) {
			fetchedCoverUrl.value = '';
			return;
		}

		// Try to fetch cover from cache/APIs
		try {
			const url = await fetchAlbumCover(track.artist, track.album);
			if (playerStore.currentTrack?.path !== newPath) {
				return;
			}

			if (playerStore.currentTrack?.cover_data_url) {
				fetchedCoverUrl.value = '';
				return;
			}

			fetchedCoverUrl.value = url;

			if (url && !track.cover_data_url) {
				const dominantColor = await extractDominantColorFromDataUrl(url);
				if (playerStore.currentTrack?.path === newPath) {
					playerStore.setCurrentTrackVisuals(url, dominantColor);
				}
			}
		} catch (error) {
			console.debug('Failed to fetch cover for current track in miniplayer');
			fetchedCoverUrl.value = '';
		}
	},
	{ immediate: true }
);

const togglePlayback = async () => {
	await playerStore.togglePlayPause();
};

const openMainWindow = async () => {
	await toggleMainAndMiniPlayer();
};

onMounted(async () => {
	await playerStore.initProgressListener();
	await playerStore.syncPlaybackSnapshot();
});

onUnmounted(() => {
	playerStore.disposeProgressListener();
});
</script>

<template>
	<div class="relative h-screen w-screen overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/90 p-3">
		<PlayerBackground />

		<div class="relative z-10 flex h-full flex-col">
			<div class="flex min-h-0 flex-1 items-center gap-2">
				<TrackMetaCard
					class="min-w-0 flex-1"
					:title="trackTitle"
					:subtitle="trackSubtitle"
					:cover-src="coverSrc"
					placeholder-text="VR"
					title-class="text-primary"
				/>

				<MiniTransportControls
					:has-track="playerStore.hasTrack"
					:has-next-track="playerStore.hasNextTrack"
					:is-playing="playerStore.isPlaying"
					:is-paused="playerStore.isPaused"
					:busy="playerStore.busy"
					next-label="Next"
					open-label="Volver"
					@toggle="togglePlayback"
					@next="playerStore.advanceQueue"
					@open="openMainWindow"
				/>
			</div>

			<PlaybackWaves class="mt-2" :steps="72" bar-height="h-3" :floor-paused="2" :floor-playing="4" :amplitude="7" />
		</div>
	</div>
</template>
