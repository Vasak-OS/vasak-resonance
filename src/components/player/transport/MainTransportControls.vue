<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';
import type { Repeticion } from '@/stores/playerQueue';

const props = defineProps<{
	hasTrack: boolean;
	hasNextTrack: boolean;
	busy: boolean;
	isPaused: boolean;
	nextActionLabel: string;
	repeticion: Repeticion;
	aleatorio: boolean;
}>();

const emit = defineEmits<{
	prev: [];
	toggle: [];
	next: [];
	repeat: [];
	shuffle: [];
}>();

const { t } = useI18n();

const playButtonLabel = computed(() => {
	if (!props.hasTrack) {
		return t('transport.play');
	}
	return props.isPaused ? t('transport.play') : t('transport.pause');
});

/** El icono dice cuál de los tres modos está puesto. */
const repeatButtonIcon = computed(() =>
	props.repeticion === 'uno' ? 'media-playlist-repeat-song' : 'media-playlist-repeat'
);

/**
 * Qué dice el botón.
 *
 * Nombra el estado en el que está, no el que vendría al apretarlo: es lo que lee
 * un lector de pantalla, y «Repetir» sin más no dice si está puesto.
 */
const repeatButtonLabel = computed(() => {
	if (props.repeticion === 'uno') {
		return t('transport.repeatOne');
	}
	return props.repeticion === 'todo' ? t('transport.repeatAll') : t('transport.repeatNone');
});

const shuffleButtonLabel = computed(() =>
	props.aleatorio ? t('transport.shuffleOn') : t('transport.shuffleOff')
);

const playPauseIcon = computed(() => {
	return props.isPaused || !props.hasTrack ? 'media-playback-start' : 'media-playback-pause';
});
</script>

<template>
	<div class="grid grid-cols-5 gap-2">
		<TransportButton
			:label="shuffleButtonLabel"
			icon="media-playlist-shuffle"
			:variant="aleatorio ? 'primary' : undefined"
			:disabled="busy"
			@click="emit('shuffle')"
		/>
		<TransportButton
			:label="t('transport.previous')"
			icon="player_rew"
			:disabled="!hasTrack || busy"
			@click="emit('prev')"
		/>
		<TransportButton
			:label="playButtonLabel"
			:icon="playPauseIcon"
			variant="primary"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<TransportButton
			:label="nextActionLabel"
			icon="player_fwd"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<TransportButton
			:label="repeatButtonLabel"
			:icon="repeatButtonIcon"
			:variant="repeticion === 'ninguna' ? undefined : 'primary'"
			:disabled="busy"
			@click="emit('repeat')"
		/>
	</div>
</template>
