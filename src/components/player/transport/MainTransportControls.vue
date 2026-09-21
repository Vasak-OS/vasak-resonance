<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
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

const repeatIcon = useReactiveIcon('media-playlist-repeat');
const repeatOneIcon = useReactiveIcon('media-playlist-repeat-song');
const shuffleIcon = useReactiveIcon('media-playlist-shuffle');

/** El icono dice cuál de los tres modos está puesto. */
const repeatButtonIcon = computed(() =>
	props.repeticion === 'uno' ? repeatOneIcon.value : repeatIcon.value
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

const prevIcon = useReactiveIcon('player_rew');
const playIcon = useReactiveIcon('media-playback-start');
const pauseIcon = useReactiveIcon('media-playback-pause');
const nextIcon = useReactiveIcon('player_fwd');

const playPauseIcon = computed(() => {
	return props.isPaused || !props.hasTrack ? playIcon.value : pauseIcon.value;
});
</script>

<template>
	<div class="grid grid-cols-5 gap-2">
		<TransportButton
			:label="shuffleButtonLabel"
			:icon-src="shuffleIcon"
			:variant="aleatorio ? 'primary' : undefined"
			:disabled="busy"
			@click="emit('shuffle')"
		/>
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
		<TransportButton
			:label="repeatButtonLabel"
			:icon-src="repeatButtonIcon"
			:variant="repeticion === 'ninguna' ? undefined : 'primary'"
			:disabled="busy"
			@click="emit('repeat')"
		/>
	</div>
</template>
