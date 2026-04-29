<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { onMounted } from 'vue';
import MiniPlayer from '@/components/player/MiniPlayer.vue';
import ScanningIndicator from '@/components/layout/ScanningIndicator.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { usePlayerStore } from '@/stores/player';
import { scanDefaultMusicFolder } from '@/services/player.service';

const isMiniPlayerWindow = getCurrentWindow().label === 'mini-player';
const AUTO_SCAN_KEY = 'resonance.auto-music-scan.last-run';
const AUTO_SCAN_INTERVAL_MS = 30 * 60 * 1000;
const playerStore = usePlayerStore();

useConfigSync({ useViewTransition: true });

onMounted(() => {
	if (isMiniPlayerWindow) {
		return;
	}

	// Listen for scan completion events
	void listen('scan-complete', async () => {
		playerStore.setScanning(false);
		await playerStore.reloadLibraryTracks();
	});

	const now = Date.now();
	const last = Number(window.localStorage.getItem(AUTO_SCAN_KEY) || '0');
	if (Number.isFinite(last) && last > 0 && now - last < AUTO_SCAN_INTERVAL_MS) {
		return;
	}

	window.localStorage.setItem(AUTO_SCAN_KEY, String(now));
	playerStore.setScanning(true);
	void scanDefaultMusicFolder().catch((error) => {
		console.warn('[App] auto music scan failed', error);
		playerStore.setScanning(false);
	});
});
</script>

<template>
	<ScanningIndicator />
	<MiniPlayer v-if="isMiniPlayerWindow" />
	<RouterView v-else />
</template>
