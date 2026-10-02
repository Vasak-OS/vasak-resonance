<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import type { Repeticion } from '@/stores/playerQueue';

const props = defineProps<{
	hasTrack: boolean;
	hasNextTrack: boolean;
	busy: boolean;
	isPaused: boolean;
	nextActionLabel: string;
	repeatMode: Repeticion;
	shuffle: boolean;
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
	props.repeatMode === 'uno' ? 'media-playlist-repeat-song' : 'media-playlist-repeat'
);

/**
 * Qué dice el botón.
 *
 * Nombra el estado en el que está, no el que vendría al apretarlo: es lo que lee
 * un lector de pantalla, y «Repetir» sin más no dice si está puesto.
 */
const repeatButtonLabel = computed(() => {
	if (props.repeatMode === 'uno') {
		return t('transport.repeatOne');
	}
	return props.repeatMode === 'todo' ? t('transport.repeatAll') : t('transport.repeatNone');
});

const shuffleButtonLabel = computed(() =>
	props.shuffle ? t('transport.shuffleOn') : t('transport.shuffleOff')
);

const playPauseIcon = computed(() => {
	return props.isPaused || !props.hasTrack ? 'media-playback-start' : 'media-playback-pause';
});
</script>

<template>
	<!-- Aleatorio y repetir son alternancias: `pressed` las marca puestas (y
	     el `aria-pressed` lo dice), en lugar del relleno primario que antes
	     las confundía con el botón de reproducir. El nombre de cada botón va en
	     `aria-label` y en `title`: el icono no lleva texto propio, o un lector
	     de pantalla lo diría dos veces. -->
	<div class="grid grid-cols-5 gap-2">
		<ActionButton
			label=""
			:icon-alt="shuffleButtonLabel"
			:title="shuffleButtonLabel"
			icon="media-playlist-shuffle"
			variant="secondary"
			:pressed="shuffle"
			:disabled="busy"
			@click="emit('shuffle')"
		/>
		<ActionButton
			label=""
			:icon-alt="t('transport.previous')"
			:title="t('transport.previous')"
			icon="player_rew"
			variant="secondary"
			:disabled="!hasTrack || busy"
			@click="emit('prev')"
		/>
		<ActionButton
			label=""
			:icon-alt="playButtonLabel"
			:title="playButtonLabel"
			:icon="playPauseIcon"
			variant="primary"
			:disabled="!hasTrack || busy"
			@click="emit('toggle')"
		/>
		<ActionButton
			label=""
			:icon-alt="nextActionLabel"
			:title="nextActionLabel"
			icon="player_fwd"
			variant="secondary"
			:disabled="!hasNextTrack || busy"
			@click="emit('next')"
		/>
		<ActionButton
			label=""
			:icon-alt="repeatButtonLabel"
			:title="repeatButtonLabel"
			:icon="repeatButtonIcon"
			variant="secondary"
			:pressed="repeatMode !== 'ninguna'"
			:disabled="busy"
			@click="emit('repeat')"
		/>
	</div>
</template>
