<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';

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

const { t } = useI18n();

const playButtonLabel = computed(() => {
	if (!props.hasTrack) {
		return t('transport.play');
	}
	return props.isPaused ? t('transport.play') : t('transport.pause');
});

const prevIcon = useReactiveIcon('player_rew');
const playIcon = useReactiveIcon('media-playback-start');
const pauseIcon = useReactiveIcon('media-playback-pause');
const nextIcon = useReactiveIcon('player_fwd');

const playPauseIcon = computed(() => {
	return props.isPaused || !props.hasTrack ? playIcon.value : pauseIcon.value;
});
</script>

<template>
	<div class="grid grid-cols-3 gap-2">
		<TransportButton
			:label="t('transport.previous')"
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
