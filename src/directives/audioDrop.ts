import type { ObjectDirective } from 'vue';

interface AudioDropBindingValue {
	onFilesDropped: (paths: string[]) => void | Promise<void>;
	onDragStateChange?: (dragging: boolean) => void;
}

type DropElement = HTMLElement & {
	__audioDropCleanup__?: () => void;
};

const getPathsFromDataTransfer = (transfer: DataTransfer): string[] => {
	const paths: string[] = [];

	for (const file of Array.from(transfer.files)) {
		const maybePath = (file as File & { path?: string }).path;
		if (typeof maybePath === 'string' && maybePath.length > 0) {
			paths.push(maybePath);
		}
	}

	if (paths.length > 0) {
		return Array.from(new Set(paths));
	}

	const uriList = transfer.getData('text/uri-list');
	if (!uriList) {
		return [];
	}

	return uriList
		.split('\n')
		.map((line) => line.trim())
		.filter((line) => line.startsWith('file://'))
		.map((line) => decodeURIComponent(line.replace('file://', '')));
};

export const audioDropDirective: ObjectDirective<DropElement, AudioDropBindingValue> = {
	mounted(el, binding) {
		let dragDepth = 0;

		const notifyDragState = (value: boolean) => {
			binding.value?.onDragStateChange?.(value);
		};

		const onDragOver = (event: DragEvent) => {
			event.preventDefault();
		};

		const onDragEnter = (event: DragEvent) => {
			event.preventDefault();
			dragDepth += 1;
			notifyDragState(true);
		};

		const onDragLeave = (event: DragEvent) => {
			event.preventDefault();
			dragDepth = Math.max(0, dragDepth - 1);
			if (dragDepth === 0) {
				notifyDragState(false);
			}
		};

		const onDrop = (event: DragEvent) => {
			event.preventDefault();
			dragDepth = 0;
			notifyDragState(false);

			const transfer = event.dataTransfer;
			if (!transfer) {
				console.warn('[audioDrop] DataTransfer no está disponible');
				return;
			}

			const paths = getPathsFromDataTransfer(transfer);
			console.log('[audioDrop] Rutas extraídas:', paths);
			if (paths.length === 0) {
				console.warn('[audioDrop] No se extrajeron rutas válidas del drop');
				return;
			}

			console.log('[audioDrop] Llamando onFilesDropped con:', paths);
			void binding.value?.onFilesDropped(paths);
		};

		el.addEventListener('dragover', onDragOver);
		el.addEventListener('dragenter', onDragEnter);
		el.addEventListener('dragleave', onDragLeave);
		el.addEventListener('drop', onDrop);

		el.__audioDropCleanup__ = () => {
			el.removeEventListener('dragover', onDragOver);
			el.removeEventListener('dragenter', onDragEnter);
			el.removeEventListener('dragleave', onDragLeave);
			el.removeEventListener('drop', onDrop);
		};
	},
	unmounted(el) {
		el.__audioDropCleanup__?.();
		delete el.__audioDropCleanup__;
	},
};
