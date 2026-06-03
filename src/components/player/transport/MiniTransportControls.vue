<script setup lang="ts">
import { computed } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';

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

const nextIcon = useReactiveIcon('player_fwd');
const playIcon = useReactiveIcon('media-playback-start');
const pauseIcon = useReactiveIcon('media-playback-pause');
const openIcon = useReactiveIcon('stock_new-window');

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
