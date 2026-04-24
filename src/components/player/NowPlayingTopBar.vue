<script setup lang="ts">
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const spectrumSteps = Array.from({ length: 28 }, (_, index) => index);

const trackTitle = computed(() => {
	if (playerStore.currentTrack?.title) {
		return playerStore.currentTrack.title;
	}
	if (playerStore.currentPath) {
		const normalized = playerStore.currentPath.replace(/\\/g, '/');
		const parts = normalized.split('/');
		return parts[parts.length - 1] || 'Sin reproduccion';
	}
	return 'Sin reproduccion';
});

const progressPercent = computed(() => {
	return Math.min(100, Math.max(0, playerStore.progressPercent));
});

const formatSeconds = (value: number | null): string => {
	const safe = Math.max(0, Math.floor(value || 0));
	const minutes = Math.floor(safe / 60)
		.toString()
		.padStart(2, '0');
	const seconds = Math.floor(safe % 60)
		.toString()
		.padStart(2, '0');
	return `${minutes}:${seconds}`;
};

const barHeight = (index: number): number => {
	const phase = (index + 1) * 0.65 + playerStore.positionSeconds * 0.38;
	const wave = Math.sin(phase) * 0.5 + 0.5;
	const floor = playerStore.isPaused || !playerStore.hasTrack ? 3 : 5;
	return Math.round(floor + wave * 11);
};

const isActiveBar = (index: number): boolean => {
	const stepPercent = ((index + 1) / spectrumSteps.length) * 100;
	return stepPercent <= progressPercent.value;
};
</script>

<template>
	<section class="min-h-14 rounded-corner border border-primary/25 bg-ui-bg/90 px-3 py-2 shadow-sm">
		<div class="mb-1 flex items-center justify-between gap-2">
			<p class="text-[10px] font-semibold uppercase tracking-[0.14em] text-primary">Reproduccion</p>
			<p class="text-[11px] text-tx-muted">
				{{ formatSeconds(playerStore.positionSeconds) }} / {{ formatSeconds(playerStore.durationSeconds) }}
			</p>
		</div>

		<div class="flex items-center gap-3">
			<p class="min-w-0 w-44 shrink-0 truncate text-xs font-semibold text-tx-main">{{ trackTitle }}</p>

			<div class="min-w-0 flex-1">
				<div class="mb-1.5 flex h-4 items-end gap-0.5 overflow-hidden">
					<span
						v-for="step in spectrumSteps"
						:key="step"
						class="w-1 rounded-sm transition-all duration-200"
						:class="isActiveBar(step) ? 'bg-primary/90' : 'bg-primary/30'"
						:style="{ height: `${barHeight(step)}px` }"
					/>
				</div>

				<div class="h-1.5 overflow-hidden rounded-full bg-ui-surface/70 ring-1 ring-primary/20">
					<div
						class="h-full rounded-full bg-primary transition-all duration-200"
						:style="{ width: `${progressPercent}%` }"
					/>
				</div>
			</div>
		</div>
	</section>
</template>
