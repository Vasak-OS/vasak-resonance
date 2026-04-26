<script setup lang="ts">
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const normalizeHex = (color: string | null | undefined): string | null => {
	if (!color) {
		return null;
	}

	const value = color.trim();
	if (!/^#([0-9a-fA-F]{6})$/.test(value)) {
		return null;
	}

	return value;
};

const hexToRgba = (hex: string, alpha: number) => {
	const normalized = hex.replace('#', '');
	const r = Number.parseInt(normalized.slice(0, 2), 16);
	const g = Number.parseInt(normalized.slice(2, 4), 16);
	const b = Number.parseInt(normalized.slice(4, 6), 16);
	return `rgba(${r}, ${g}, ${b}, ${alpha})`;
};

const gradientStyle = computed(() => {
	const dominant = normalizeHex(playerStore.currentTrack?.dominant_color);
	if (!dominant) {
		return {
			background:
				'linear-gradient(to top, color-mix(in srgb, var(--color-primary) 26%, transparent) 0%, color-mix(in srgb, var(--color-secondary) 18%, transparent) 48%, transparent 100%)',
		};
	}

	return {
		background: `linear-gradient(to top, ${hexToRgba(dominant, 0.28)} 0%, ${hexToRgba(dominant, 0.16)} 48%, ${hexToRgba(dominant, 0.05)} 80%, transparent 100%)`,
	};
});
</script>

<template>
	<div class="pointer-events-none absolute inset-0 z-0 overflow-hidden rounded-corner" :style="gradientStyle" />
</template>
