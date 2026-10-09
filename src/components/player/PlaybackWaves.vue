<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Slider } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { devLog } from '@/composables/useDevLog';
import { useElementWidth } from '@/composables/useElementWidth';
import { formatSeconds } from '@/composables/useTimeFormat';
import { usePlayerStore } from '@/stores/player';

const props = withDefaults(
	defineProps<{
		steps?: number;
		barHeight?: string;
		floorPaused?: number;
		floorPlaying?: number;
		amplitude?: number;
		phaseMultiplier?: number;
		timeMultiplier?: number;
		activeClass?: string;
		inactiveClass?: string;
	}>(),
	{
		steps: 110,
		barHeight: 'h-4',
		floorPaused: 3,
		floorPlaying: 5,
		amplitude: 11,
		phaseMultiplier: 0.65,
		timeMultiplier: 0.38,
		activeClass: 'bg-secondary',
		inactiveClass: 'bg-primary/30',
	}
);

const playerStore = usePlayerStore();
const { t } = useI18n();

const progressPercent = computed(() => {
	return Math.min(100, Math.max(0, playerStore.progressPercent));
});

const totalDuration = computed(() => playerStore.durationSeconds ?? 0);

const sliderValue = computed(() =>
	totalDuration.value > 0 ? Math.round((progressPercent.value / 100) * totalDuration.value) : 0
);

const commitSeek = (seconds: number) => {
	devLog('[PlaybackWaves] seek:', seconds);
	playerStore.positionSeconds = seconds;
	playerStore.seekTo(seconds);
};

// Mientras se arrastra se mueve sólo la posición que se muestra; el salto de
// verdad va al soltar, o el hilo de audio recibiría un pedido por píxel.
const onSliderInput = (seconds: number) => {
	playerStore.positionSeconds = seconds;
};

const onSliderChange = (seconds: number) => {
	commitSeek(seconds);
};

const seekValueText = computed(
	() => `${formatSeconds(sliderValue.value)} / ${formatSeconds(totalDuration.value)}`
);

/**
 * Cuántas barras entran en el ancho que hay.
 *
 * Cada barra necesita su hueco de 2 píxeles y al menos otros 2 de ancho: con
 * las 110 de siempre en una ventana de 240, los huecos solos ocupaban más que
 * la tira y las barras quedaban en cero —la onda desaparecía—. Con lugar,
 * son las de siempre.
 */
const MIN_PIXELS_PER_BAR = 4;
const strip = ref<HTMLElement | null>(null);
const stripWidth = useElementWidth(strip);
const visibleSteps = computed(() =>
	stripWidth.value > 0
		? Math.max(8, Math.min(props.steps, Math.floor(stripWidth.value / MIN_PIXELS_PER_BAR)))
		: props.steps
);

const bars = computed(() => {
	const steps = visibleSteps.value;
	const amp = props.amplitude;
	const phaseMul = props.phaseMultiplier;
	const timeMul = props.timeMultiplier;
	const floor =
		playerStore.isPaused || !playerStore.hasTrack ? props.floorPaused : props.floorPlaying;
	const pos = playerStore.positionSeconds;
	const pct = progressPercent.value;
	const result = new Array(steps);

	for (let i = 0; i < steps; i++) {
		const phase = (i + 1) * phaseMul + pos * timeMul;
		const wave = Math.sin(phase) * 0.5 + 0.5;
		const stepPercent = ((i + 1) / steps) * 100;
		result[i] = {
			height: Math.round(floor + wave * amp),
			isActive: stepPercent <= pct,
		};
	}

	return result;
});
</script>

<template>
	<div
		ref="strip"
		class="flex w-full items-end gap-0.5 overflow-hidden"
		:class="barHeight"
	>
		<span
			v-for="(bar, step) in bars"
			:key="step"
			class="flex-1 rounded-corner-xs transition-[height,background-color] duration-200"
			:class="bar.isActive ? activeClass : inactiveClass"
			:style="{ height: `${bar.height}px` }"
		/>
	</div>
	<!-- El deslizador de la librería, con nombre y con el tiempo en
	     `aria-valuetext`: el `range` de antes no decía qué movía. Su zona de
	     toque mide 32 píxeles y el `range` nativo 16 en su línea de 27: los
	     márgenes negativos le devuelven a la barra el alto de antes (medido en
	     el banco: 204 píxeles la barra entera) sin achicar dónde se puede
	     apretar. -->
	<Slider
		v-if="totalDuration > 0"
		class="-mt-0.5 -mb-[3px] w-full"
		:model-value="sliderValue"
		:min="0"
		:max="totalDuration"
		:label="t('player.seek')"
		:value-text="seekValueText"
		@update:model-value="onSliderInput"
		@change="onSliderChange"
	/>
</template>
