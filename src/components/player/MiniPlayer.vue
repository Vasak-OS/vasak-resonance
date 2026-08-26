<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import PlayerBackground from '@/components/player/PlayerBackground.vue';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MiniTransportControls from '@/components/player/transport/MiniTransportControls.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { fetchAlbumCover } from '@/services/album-cover.service';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
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
			// La portada trae su color: lo calculó Rust con los bytes que ya
			// tenía, en lugar de que esta ventana decodifique la imagen otra vez.
			const portada = await fetchAlbumCover(track.artist, track.album);
			if (playerStore.currentTrack?.path !== newPath) {
				return;
			}

			if (playerStore.currentTrack?.cover_data_url) {
				fetchedCoverUrl.value = '';
				return;
			}

			fetchedCoverUrl.value = portada.cover_data_url;

			if (portada.cover_data_url && !track.cover_data_url) {
				playerStore.setCurrentTrackVisuals(portada.cover_data_url, portada.dominant_color || null);
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
					:next-label="t('transport.next')"
					:open-label="t('miniPlayer.backToWindow')"
					@toggle="togglePlayback"
					@next="playerStore.advanceQueue"
					@open="openMainWindow"
				/>
			</div>

			<PlaybackWaves class="mt-2" :steps="72" bar-height="h-3" :floor-paused="2" :floor-playing="4" :amplitude="7" />
		</div>
	</div>
</template>
