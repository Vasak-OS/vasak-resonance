import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { onMounted, onUnmounted, type Ref, ref } from 'vue';

export function useReactiveIcon(name: string, type: 'symbol' | 'icon' = 'symbol'): Ref<string> {
	const iconSrc = ref('');

	const fetchIcon = async () => {
		try {
			if (type === 'icon') {
				iconSrc.value = await getIconSource(name);
			} else {
				iconSrc.value = await getSymbolSource(name);
			}
		} catch {
			iconSrc.value = '';
		}
	};

	let unlisten: UnlistenFn | null = null;

	onMounted(async () => {
		await fetchIcon();
		unlisten = await listen('vicons:theme-changed', fetchIcon);
	});

	onUnmounted(() => {
		if (unlisten) {
			unlisten();
		}
	});

	return iconSrc;
}
