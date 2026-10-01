import { onBeforeUnmount, onMounted, type Ref, ref } from 'vue';

/**
 * El ancho de un elemento, siguiéndolo.
 *
 * Con un `ResizeObserver` sobre el elemento y no sobre la pantalla: en el
 * WebView de WebKitGTK ni `matchMedia` ni el evento `resize` llegan, así que
 * algo que dependa de ellos se queda con el ancho del arranque para siempre.
 * Cero mientras no se midió —la primera pasada, o un entorno sin maquetado—.
 */
export function useElementWidth(target: Ref<HTMLElement | null>): Ref<number> {
	const width = ref(0);
	let observer: ResizeObserver | null = null;

	const measure = () => {
		width.value = target.value?.clientWidth ?? 0;
	};

	onMounted(() => {
		measure();
		// Sin `ResizeObserver` —un entorno de pruebas sin DOM completo— queda
		// el ancho del arranque, que es mejor que romper.
		if (typeof ResizeObserver === 'function' && target.value) {
			observer = new ResizeObserver(measure);
			observer.observe(target.value);
		}
	});

	onBeforeUnmount(() => {
		observer?.disconnect();
		observer = null;
	});

	return width;
}
