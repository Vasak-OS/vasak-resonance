<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';

const props = defineProps<{
	hasTrack: boolean;
	hasNextTrack: boolean;
	isPlaying: boolean;
	isPaused: boolean;
	busy: boolean;
	nextLabel?: string;
	openLabel?: string;
}>();

const emit = defineEmits<{
	toggle: [];
	next: [];
	open: [];
}>();

const toggleLabel = computed(() => {
	return props.isPaused || !props.isPlaying ? 'Play' : 'Pause';
});

const nextIcon = ref('');
const playIcon = ref('');
const pauseIcon = ref('');
const openIcon = ref('');

onMounted(async () => {
	const getSymbolSrc = getSymbolSource;
	const [nextSrc, playSrc, pauseSrc, openSrc] = await Promise.all([
		getSymbolSrc('player_fwd').catch(() => ''),
		getSymbolSrc('media-playback-start').catch(() => ''),
		getSymbolSrc('media-playback-pause').catch(() => ''),
		getSymbolSrc('stock_new-window').catch(() => ''),
	]);

	nextIcon.value = nextSrc;
	playIcon.value = playSrc;
	pauseIcon.value = pauseSrc;
	openIcon.value = openSrc;
});

const playPauseIcon = computed(() => {
	return props.isPaused || !props.isPlaying ? playIcon.value : pauseIcon.value;
});
</script>

<template>
	<div class="flex shrink-0 items-center gap-1">
		<TransportButton
			:label="nextLabel || 'Next'"
			:icon-src="nextIcon"
			size="sm"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<TransportButton
			:label="toggleLabel"
			:icon-src="playPauseIcon"
			variant="primary"
			size="sm"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="openLabel || 'Abrir'"
			:icon-src="openIcon"
			size="sm"
			@click="emit('open')"
		/>
	</div>
</template>
