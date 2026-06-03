import { getCurrentWebview } from '@tauri-apps/api/webview';
import { onMounted, onUnmounted, ref } from 'vue';
import { devLog } from '@/composables/useDevLog';

interface AudioDropOptions {
	onFilesDropped: (paths: string[]) => void | Promise<void>;
	onDragStateChange?: (dragging: boolean) => void;
}

export function useAudioDrop(options: AudioDropOptions) {
	const isDragging = ref(false);
	let unlistenDrop: (() => void) | null = null;

	onMounted(() => {
		getCurrentWebview()
			.onDragDropEvent((event) => {
				devLog('[useAudioDrop] Event:', event.payload.type, event.payload);

				if (event.payload.type === 'enter') {
					const paths = (event.payload.paths as string[]) ?? [];
					devLog('[useAudioDrop] Drag enter, paths:', paths);
					if (paths.length > 0) {
						isDragging.value = true;
						options.onDragStateChange?.(true);
					}
				} else if (event.payload.type === 'over') {
				} else if (event.payload.type === 'leave') {
					devLog('[useAudioDrop] Drag leave');
					isDragging.value = false;
					options.onDragStateChange?.(false);
				} else if (event.payload.type === 'drop') {
					const paths = (event.payload.paths as string[]) ?? [];
					devLog('[useAudioDrop] Drop detected, paths:', paths);
					isDragging.value = false;
					options.onDragStateChange?.(false);

					if (paths.length > 0) {
						void options.onFilesDropped(paths);
					}
				}
			})
			.then((unlisten) => {
				unlistenDrop = unlisten;
				devLog('[useAudioDrop] Listener registered');
			})
			.catch((error) => {
				console.error('[useAudioDrop] Error registering listener:', error);
			});
	});

	onUnmounted(() => {
		if (unlistenDrop) {
			unlistenDrop();
			console.log('[useAudioDrop] Listener unregistered');
		}
	});

	return {
		isDragging,
	};
}
