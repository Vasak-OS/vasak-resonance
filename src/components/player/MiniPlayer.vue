<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { computed, onMounted, onUnmounted } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { usePlayerStore } from '@/stores/player';
import { toggleMainAndMiniPlayer } from '@/services/window.service';

const playerStore = usePlayerStore();
let unlistenConfig: UnlistenFn | null = null;

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
});

const coverSrc = computed(() => playerStore.currentTrack?.cover_data_url || '');

const togglePlayback = async () => {
	await playerStore.togglePlayPause();
};

const openMainWindow = async () => {
	await toggleMainAndMiniPlayer();
};

onMounted(async () => {
	const configStore = useConfigStore() as Store<
		'config',
		{ config: any; loadConfig: () => Promise<void> }
	>;

	await configStore.loadConfig();

	unlistenConfig = await listen('config-changed', async () => {
		await configStore.loadConfig();
	});

	await playerStore.initProgressListener();
	await playerStore.syncPlaybackSnapshot();
});

onUnmounted(() => {
	if (unlistenConfig) {
		unlistenConfig();
		unlistenConfig = null;
	}

	playerStore.disposeProgressListener();
});
</script>

<template>
	<div class="h-screen w-screen overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/90 p-3">
		<div class="flex h-full flex-col">
			<div class="flex min-h-0 flex-1 items-center gap-3">
				<div class="h-16 w-16 shrink-0 overflow-hidden rounded-corner border border-primary/25 bg-ui-bg/70">
					<img v-if="coverSrc" :src="coverSrc" alt="Caratula" class="h-full w-full object-cover">
					<div v-else class="flex h-full w-full items-center justify-center text-[10px] text-tx-muted">
						Sin portada
					</div>
				</div>

				<div class="min-w-0 flex-1">
					<p class="truncate text-xs uppercase tracking-[0.14em] text-tx-muted">MiniPlayer</p>
					<p class="truncate text-sm font-semibold text-primary">{{ trackTitle }}</p>
					<p class="truncate text-xs text-tx-muted">
						{{ playerStore.currentTrack?.artist || 'Unknown Artist' }}
					</p>
				</div>

				<div class="flex shrink-0 items-center gap-2">
					<button
						class="rounded-corner border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-semibold text-primary transition-colors hover:bg-primary/20"
						@click="togglePlayback"
					>
						{{ playerStore.isPaused || !playerStore.isPlaying ? 'Play' : 'Pause' }}
					</button>
					<button
						class="rounded-corner border border-ui-border bg-ui-bg/80 px-3 py-1.5 text-xs font-semibold text-tx-main transition-colors hover:bg-ui-bg"
						@click="openMainWindow"
					>
						Abrir
					</button>
				</div>
			</div>

			<PlaybackWaves class="mt-2" :steps="72" bar-height="h-3" :floor-paused="2" :floor-playing="4" :amplitude="7" />
		</div>
	</div>
</template>
