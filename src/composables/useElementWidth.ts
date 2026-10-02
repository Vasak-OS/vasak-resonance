import { type Ref, ref, watch } from 'vue';

/**
 * El ancho de un elemento, siguiéndolo.
 *
 * Con un `ResizeObserver` sobre el elemento y no sobre la pantalla: en el
 * WebView de WebKitGTK ni `matchMedia` ni el evento `resize` llegan, así que
 * algo que dependa de ellos se queda con el ancho del arranque para siempre.
 * Cero mientras no se midió —la primera pasada, o un entorno sin maquetado—.
 *
 * Se mira la referencia y no sólo el montaje: la lista de Inicio o la de
 * Favoritos aparece **después**, cuando termina de cargar (`v-else`), y medida
 * al montar se quedaba en cero —y con los botones con texto en una fila
 * angosta—. Cada elemento nuevo se mide y se observa; el anterior se suelta.
 */
export function useElementWidth(target: Ref<HTMLElement | null>): Ref<number> {
	const width = ref(0);

	watch(
		target,
		(element, _previous, onCleanup) => {
			width.value = element?.clientWidth ?? 0;
			// Sin `ResizeObserver` —un entorno de pruebas sin DOM completo— queda
			// el ancho de cuando apareció, que es mejor que romper.
			if (!element || typeof ResizeObserver !== 'function') {
				return;
			}
			const observer = new ResizeObserver(() => {
				width.value = element.clientWidth;
			});
			observer.observe(element);
			onCleanup(() => observer.disconnect());
		},
		{ flush: 'post', immediate: true }
	);

	return width;
}
