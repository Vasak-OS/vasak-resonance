<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { usePlayerStore } from '@/stores/player';
import { toggleMainAndMiniPlayer } from '@/services/window.service';

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
			<div class="flex min-h-0 flex-1 items-center gap-3">
				<TrackMetaCard
					:title="trackTitle"
					:subtitle="trackSubtitle"
					:cover-src="coverSrc"
					placeholder-text="VR"
					title-class="text-primary"
				/>

				<div class="flex shrink-0 items-center gap-2">
					<button
						class="rounded-corner border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-semibold text-primary transition-colors hover:bg-primary/20"
						@click="togglePlayback"
					>
						{{ playerStore.isPaused || !playerStore.isPlaying ? 'Play' : 'Pause' }}
					</button>
					<button
						class="rounded-corner border border-ui-border bg-ui-bg/80 px-3 py-1.5 text-xs font-semibold text-tx-main transition-colors hover:bg-ui-bg"
						@click="openMainWindow"
					>
						Abrir
					</button>
				</div>
			</div>

			<PlaybackWaves class="mt-2" :steps="72" bar-height="h-3" :floor-paused="2" :floor-playing="4" :amplitude="7" />
		</div>
	</div>
</template>
