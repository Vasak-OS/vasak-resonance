import { computed, type Ref } from 'vue';
import { useElementWidth } from '@/composables/useElementWidth';
import { isNarrowRow } from '@/tools/window-layout';

/** Si la fila de la ventana es angosta: una columna por vez. Ver `window-layout.ts`. */
export function useNarrowRow(target: Ref<HTMLElement | null>): Ref<boolean> {
	const width = useElementWidth(target);
	return computed(() => isNarrowRow(width.value));
}
