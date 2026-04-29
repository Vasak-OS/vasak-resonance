<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MiniTransportControls from '@/components/player/transport/MiniTransportControls.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

useConfigSync();

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});

const trackSubtitle = useTrackSubtitle({
	currentTrack: () => playerStore.currentTrack,
});

const coverSrc = computed(() => playerStore.currentTrack?.cover_data_url || '');

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
	<div class="h-screen w-screen overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/90 p-3">
		<div class="flex h-full flex-col">
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
