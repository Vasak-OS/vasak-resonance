<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';

const props = defineProps<{
	hasTrack: boolean;
	hasNextTrack: boolean;
	busy: boolean;
	isPaused: boolean;
	nextActionLabel: string;
}>();

const emit = defineEmits<{
	prev: [];
	toggle: [];
	next: [];
}>();

const playButtonLabel = computed(() => {
	if (!props.hasTrack) {
		return 'Play';
	}
	return props.isPaused ? 'Play' : 'Pause';
});

const prevIcon = ref('');
const playIcon = ref('');
const pauseIcon = ref('');
const nextIcon = ref('');

onMounted(async () => {
	const getSymbolSrc = getSymbolSource;
	const [prevSrc, playSrc, pauseSrc, nextSrc] = await Promise.all([
		getSymbolSrc('player_rew').catch(() => ''),
		getSymbolSrc('media-playback-start').catch(() => ''),
		getSymbolSrc('media-playback-pause').catch(() => ''),
		getSymbolSrc('player_fwd').catch(() => ''),
	]);

	prevIcon.value = prevSrc;
	playIcon.value = playSrc;
	pauseIcon.value = pauseSrc;
	nextIcon.value = nextSrc;
});

const playPauseIcon = computed(() => {
	return props.isPaused || !props.hasTrack ? playIcon.value : pauseIcon.value;
});
</script>

<template>
	<div class="grid grid-cols-3 gap-2">
		<TransportButton
			label="Prev"
			:icon-src="prevIcon"
			:disabled="!hasTrack || busy"
			@click="emit('prev')"
		/>
		<TransportButton
			:label="playButtonLabel"
			:icon-src="playPauseIcon"
			variant="primary"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="nextActionLabel"
			:icon-src="nextIcon"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
	</div>
</template>
