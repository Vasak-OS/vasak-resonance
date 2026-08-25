<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onMounted } from 'vue';
import ScanningIndicator from '@/components/layout/ScanningIndicator.vue';
import MiniPlayer from '@/components/player/MiniPlayer.vue';
import { useConfigSync } from '@/composables/useConfigSync';
import { useTextFieldContextMenu } from '@/composables/useTextFieldContextMenu';
import { scanDefaultMusicFolder } from '@/services/player.service';
import { usePlayerStore } from '@/stores/player';
import { useSettingsStore } from '@/stores/settings';

const isMiniPlayerWindow = getCurrentWindow().label === 'mini-player';
const AUTO_SCAN_KEY = 'resonance.auto-music-scan.last-run';
const AUTO_SCAN_INTERVAL_MS = 30 * 60 * 1000;
const playerStore = usePlayerStore();
const settingsStore = useSettingsStore();

useConfigSync({ useViewTransition: true });

// El clic derecho de todos los campos de texto de la ventana, enganchado una
// sola vez.
useTextFieldContextMenu();

onMounted(() => {
	if (isMiniPlayerWindow) {
		return;
	}

	// Antes de cualquier otra cosa: el hilo de audio arranca con su propio valor
	// por omisión, así que sin esto alguien que apagó el encadenado lo volvería a
	// escuchar en cada arranque. Va acá y no en la vista de Ajustes porque el
	// ajuste tiene que valer desde la primera canción, no desde la primera vez
	// que se abre esa pantalla.
	void settingsStore.load();

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
