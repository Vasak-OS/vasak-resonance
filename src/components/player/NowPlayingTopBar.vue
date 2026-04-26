<script setup lang="ts">
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
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
</script>

<template>
	<section class="min-h-14 rounded-corner border border-primary/25 bg-ui-bg/90 px-3 py-2 shadow-sm">
		<div class="flex items-center gap-3">
			<div class="min-w-0 flex-1">
				<PlaybackWaves class="mb-1.5" />
			</div>
		</div>

    <div class="mt-1 flex items-center justify-between gap-2">
      <p class="min-w-0 truncate text-[10px] font-semibold uppercase tracking-[0.14em] text-primary">
        {{ trackTitle }}
      </p>
      <p class="text-[11px] text-tx-muted">
        {{ formatSeconds(playerStore.positionSeconds) }} / {{ formatSeconds(playerStore.durationSeconds) }}
      </p>
    </div>

	</section>
</template>
