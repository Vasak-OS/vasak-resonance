import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { onMounted, onUnmounted } from 'vue';

interface UseConfigSyncOptions {
	useViewTransition?: boolean;
}

let sharedUnlisten: UnlistenFn | null = null;
let activeConsumers = 0;
let activeLoad: Promise<void> | null = null;

const loadConfigSafely = async (configStore: Store<'config', { config: any; loadConfig: () => Promise<void> }>) => {
	if (!activeLoad) {
		activeLoad = configStore.loadConfig().finally(() => {
			activeLoad = null;
		});
	}

	await activeLoad;
};

export const useConfigSync = ({ useViewTransition = false }: UseConfigSyncOptions = {}) => {
	onMounted(async () => {
		activeConsumers += 1;

		const configStore = useConfigStore() as Store<
			'config',
			{ config: any; loadConfig: () => Promise<void> }
		>;

		await loadConfigSafely(configStore);

		if (sharedUnlisten) {
			return;
		}

		sharedUnlisten = await listen('config-changed', async () => {
			if (useViewTransition && 'startViewTransition' in document) {
				document.startViewTransition(() => {
					void configStore.loadConfig();
				});
				return;
			}

			await configStore.loadConfig();
		});
	});

	onUnmounted(() => {
		activeConsumers = Math.max(0, activeConsumers - 1);

		if (activeConsumers === 0 && sharedUnlisten) {
			sharedUnlisten();
			sharedUnlisten = null;
		}
	});
};
