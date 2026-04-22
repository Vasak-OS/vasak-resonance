<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import PlayerPlaybackControls from '@/components/player/PlayerPlaybackControls.vue';
import PlayerQueuePanel from '@/components/player/PlayerQueuePanel.vue';
import PlayerSummaryCard from '@/components/player/PlayerSummaryCard.vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const seekModel = ref(0);
const volumeModel = ref(1);

const coverArt = computed(() => playerStore.currentTrack?.cover_data_url || '');

const trackTitle = computed(() => {
	if (playerStore.currentTrack?.title) {
		return playerStore.currentTrack.title;
	}
	if (playerStore.currentPath) {
		const parts = playerStore.currentPath.split('/');
		return parts[parts.length - 1] || 'Unknown track';
	}
	return 'Arrastra una canción para reproducir';
});

const trackSubtitle = computed(() => {
	if (!playerStore.currentTrack) {
		return 'Vasak Resonance';
	}
	const artist = playerStore.currentTrack.artist || 'Unknown Artist';
	const album = playerStore.currentTrack.album || 'Unknown Album';
	return `${artist} • ${album}`;
});

const playButtonLabel = computed(() => {
	if (!playerStore.hasTrack) {
		return 'Play';
	}
	return playerStore.isPaused ? 'Resume' : 'Pause';
});

const queueLabel = computed(() => {
	const count = playerStore.queuedCount;
	if (count <= 0) {
		return '';
	}
	return count === 1 ? '1 en cola' : `${count} en cola`;
});

const queueItems = computed(() => playerStore.queue);

const onSeekCommit = async () => {
	await playerStore.seekTo(seekModel.value);
};

const onVolumeCommit = async () => {
	await playerStore.setVolume(volumeModel.value);
};

onMounted(async () => {
	await playerStore.initProgressListener();
	await playerStore.initMprisNextListener();
	await playerStore.initMprisPreviousListener();
	await playerStore.initMprisStopListener();
});

onUnmounted(() => {
	playerStore.disposeProgressListener();
	playerStore.disposeMprisNextListener();
	playerStore.disposeMprisPreviousListener();
	playerStore.disposeMprisStopListener();
});

watch(
	() => playerStore.positionSeconds,
	(value) => {
		seekModel.value = value;
	},
	{ immediate: true }
);

watch(
	() => playerStore.volume,
	(value) => {
		volumeModel.value = value;
	},
	{ immediate: true }
);
</script>

<template>
	<section class="relative mx-auto flex w-full max-w-6xl flex-col gap-4 overflow-hidden rounded-[calc(var(--corner-radius)+10px)] border border-ui-border bg-ui-bg/80 p-4 shadow-[0_20px_60px_rgba(0,0,0,0.28)] backdrop-blur-sm">
		<div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_15%_10%,rgba(221,120,120,0.14)_0%,transparent_30%),radial-gradient(circle_at_85%_85%,rgba(136,57,239,0.12)_0%,transparent_28%)]" />

		<div class="relative grid gap-4 lg:grid-cols-[minmax(0,1.12fr)_minmax(0,0.88fr)]">
			<PlayerSummaryCard
				:cover-art="coverArt"
				:title="trackTitle"
				:subtitle="trackSubtitle"
				:queue-label="queueLabel"
			/>

			<PlayerPlaybackControls
				v-model:seek-value="seekModel"
				v-model:volume-value="volumeModel"
				:has-track="playerStore.hasTrack"
				:busy="playerStore.busy"
				:is-paused="playerStore.isPaused"
				:play-label="playButtonLabel"
				:position-seconds="playerStore.positionSeconds"
				:duration-seconds="playerStore.durationSeconds"
				@toggle-play-pause="playerStore.togglePlayPause"
				@seek-commit="onSeekCommit"
				@volume-commit="onVolumeCommit"
			/>
		</div>

		<PlayerQueuePanel
			v-if="queueItems.length > 0"
			:queue-items="queueItems"
			@clear="playerStore.clearQueue"
			@remove="playerStore.removeQueueItem"
			@reorder="playerStore.moveQueueItem"
		/>

		<p
			v-if="playerStore.error"
			class="relative rounded-[var(--corner-radius)] border border-status-danger/35 bg-status-danger/10 px-3 py-2 text-sm text-status-danger"
		>
			{{ playerStore.error }}
		</p>
	</section>
</template>
