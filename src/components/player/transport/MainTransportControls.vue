<script setup lang="ts">
import { computed } from 'vue';
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
</script>

<template>
	<div class="grid grid-cols-3 gap-2">
		<TransportButton
			label="Prev"
			:disabled="!hasTrack || busy"
			@click="emit('prev')"
		/>
		<TransportButton
			:label="playButtonLabel"
			variant="primary"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="nextActionLabel"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
	</div>
</template>
