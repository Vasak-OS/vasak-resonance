<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
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

const { t } = useI18n();

const toggleLabel = computed(() => {
	return props.isPaused || !props.isPlaying ? t('transport.play') : t('transport.pause');
});

const playPauseIcon = computed(() => {
	return props.isPaused || !props.isPlaying ? 'media-playback-start' : 'media-playback-pause';
});
</script>

<template>
	<div class="flex shrink-0 items-center gap-1">
		<TransportButton
			:label="nextLabel || t('transport.next')"
			icon="player_fwd"
			size="sm"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<TransportButton
			:label="toggleLabel"
			:icon="playPauseIcon"
			variant="primary"
			size="sm"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="openLabel || t('transport.open')"
			icon="stock_new-window"
			size="sm"
			@click="emit('open')"
		/>
	</div>
</template>
