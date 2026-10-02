<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';

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
		<ActionButton
			label=""
			:icon-alt="nextLabel || t('transport.next')"
			:title="nextLabel || t('transport.next')"
			icon="player_fwd"
			variant="secondary"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<ActionButton
			label=""
			:icon-alt="toggleLabel"
			:title="toggleLabel"
			:icon="playPauseIcon"
			variant="primary"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<ActionButton
			label=""
			:icon-alt="openLabel || t('transport.open')"
			:title="openLabel || t('transport.open')"
			icon="stock_new-window"
			variant="secondary"
			@click="emit('open')"
		/>
	</div>
</template>
