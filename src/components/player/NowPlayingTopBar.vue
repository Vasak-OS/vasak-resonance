<script setup lang="ts">
import LyricsView from '@/components/player/LyricsView.vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import VolumeControl from '@/components/player/VolumeControl.vue';
import { formatSeconds } from '@/composables/useTimeFormat';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});
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
			<VolumeControl />
      <p class="text-[11px] text-tx-muted">
				<template v-if="playerStore.isStream">
					{{ formatSeconds(playerStore.positionSeconds) }}
				</template>
				<template v-else>
					{{ formatSeconds(playerStore.positionSeconds) }} / {{ formatSeconds(playerStore.durationSeconds) }}
				</template>
      </p>
    </div>

		<div class="mt-2">
			<LyricsView />
		</div>

	</section>
</template>
