<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { onMounted, onUnmounted, type Ref, ref } from 'vue';
import MiniPlayer from '@/components/player/MiniPlayer.vue';

let unListenConfig: Ref<UnlistenFn | null> = ref(null);
const isMiniPlayerWindow = getCurrentWindow().label === 'mini-player';

onMounted(async () => {
	try {
		const configStore = useConfigStore() as Store<
			'config',
			{ config: any; loadConfig: () => Promise<void> }
		>;
		await configStore.loadConfig();

		unListenConfig.value = await listen('config-changed', async () => {
			if ('startViewTransition' in document) {
				document.startViewTransition(() => {
					configStore.loadConfig();
				});
				return;
			}

			await configStore.loadConfig();
		});
	} catch (error: any) {
		console.error('Error al cargar configuración en App.vue', error);
	}
});

onUnmounted(() => {
	if (unListenConfig.value !== null) {
		unListenConfig.value();
	}
});
</script>

<template>
	<MiniPlayer v-if="isMiniPlayerWindow" />
	<RouterView v-else />
</template>
