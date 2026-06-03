<script setup lang="ts">
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';
import { devLog } from '@/composables/useDevLog';

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

const progressPercent = computed(() => {
	return Math.min(100, Math.max(0, playerStore.progressPercent));
});

const totalDuration = computed(() => playerStore.durationSeconds ?? 0);

const onClick = (e: MouseEvent) => {
	const target = e.currentTarget as HTMLElement;
	if (!target) return;
	const dur = totalDuration.value;
	if (dur <= 0) return;
	const rect = target.getBoundingClientRect();
	const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
	const seconds = Math.round(pct * dur);
	devLog('[PlaybackWaves] click seek:', { dur, pct, seconds });
	playerStore.seekTo(seconds);
};

const bars = computed(() => {
	const steps = props.steps;
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
		class="flex w-full cursor-pointer items-end gap-0.5 overflow-hidden"
		:class="barHeight"
		@click="onClick"
	>
		<span
			v-for="(bar, step) in bars"
			:key="step"
			class="flex-1 rounded-sm transition-all duration-200"
			:class="bar.isActive ? activeClass : inactiveClass"
			:style="{ height: `${bar.height}px` }"
		/>
	</div>
</template>
