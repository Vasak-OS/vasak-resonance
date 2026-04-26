<script setup lang="ts">
import { computed } from 'vue';
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
</script>

<template>
	<div class="flex shrink-0 items-center gap-2">
		<TransportButton
			:label="nextLabel || 'Next'"
			size="sm"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<TransportButton
			:label="toggleLabel"
			variant="primary"
			size="sm"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="openLabel || 'Abrir'"
			size="sm"
			@click="emit('open')"
		/>
	</div>
</template>
