<script setup lang="ts">
import { computed } from 'vue';
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

const spectrumSteps = computed(() => Array.from({ length: props.steps }, (_, index) => index));

const progressPercent = computed(() => {
	return Math.min(100, Math.max(0, playerStore.progressPercent));
});

const barHeight = (index: number): number => {
	const phase = (index + 1) * props.phaseMultiplier + playerStore.positionSeconds * props.timeMultiplier;
	const wave = Math.sin(phase) * 0.5 + 0.5;
	const floor = playerStore.isPaused || !playerStore.hasTrack ? props.floorPaused : props.floorPlaying;
	return Math.round(floor + wave * props.amplitude);
};

const isActiveBar = (index: number): boolean => {
	const stepPercent = ((index + 1) / props.steps) * 100;
	return stepPercent <= progressPercent.value;
};
</script>

<template>
	<div class="flex w-full items-end gap-0.5 overflow-hidden" :class="barHeight">
		<span
			v-for="step in spectrumSteps"
			:key="step"
			class="flex-1 rounded-sm transition-all duration-200"
			:class="isActiveBar(step) ? activeClass : inactiveClass"
			:style="{ height: `${barHeight(step)}px` }"
		/>
	</div>
</template>