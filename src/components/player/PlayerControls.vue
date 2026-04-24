<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import PlayerQueuePanel from '@/components/player/PlayerQueuePanel.vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const seekModel = ref(0);
const volumeModel = ref(1);

const playButtonLabel = computed(() => {
	if (!playerStore.hasTrack) {
		return 'Play';
	}
	return playerStore.isPaused ? 'Resume' : 'Pause';
});

const queueItems = computed(() => playerStore.queue);

const onSeekCommit = async () => {
	await playerStore.seekTo(seekModel.value);
};

const onVolumeCommit = async () => {
	await playerStore.setVolume(volumeModel.value);
};

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
	<section class="relative mx-auto flex w-full max-w-6xl flex-col gap-4 overflow-hidden p-4">

		<PlayerQueuePanel
			v-if="queueItems.length > 0"
			:queue-items="queueItems"
			@clear="playerStore.clearQueue"
			@remove="playerStore.removeQueueItem"
			@reorder="playerStore.moveQueueItem"
		/>

		<p
			v-if="playerStore.error"
			class="relative rounded-corner border border-status-danger/35 bg-status-danger/10 px-3 py-2 text-sm text-status-danger"
		>
			{{ playerStore.error }}
		</p>
	</section>
</template>
