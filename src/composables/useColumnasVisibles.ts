import { onBeforeUnmount, onMounted, type Ref, ref } from 'vue';

/**
 * Cuántas columnas dibuja una cuadrícula al ancho que hay ahora.
 *
 * Hace falta porque el `RecycleScroller` necesita que le **digan** cuántas
 * columnas hay: coloca los elementos él, así que no puede enterarse de lo que
 * decidió el CSS. Los cortes que se le pasan son los mismos que traía la clase
 * de Tailwind, para que la cuadrícula se vea igual que antes.
 */

/** Un corte: desde este ancho, tantas columnas. */
export interface Corte {
	desde: number;
	columnas: number;
}

/**
 * Las columnas que corresponden a un ancho.
 *
 * Gana el corte más alto que el ancho alcance, así que el orden en que vengan
 * no importa. Sin ningún corte alcanzado, una columna: es lo que hace
 * `grid-cols-1`.
 */
export function columnasPara(ancho: number, cortes: Corte[]): number {
	return cortes
		.filter((corte) => ancho >= corte.desde)
		.reduce((mayor, corte) => Math.max(mayor, corte.columnas), 1);
}

/**
 * Las columnas de ahora, siguiendo al ancho de la ventana.
 *
 * Mira el elemento raíz con un `ResizeObserver` y **no** `matchMedia` ni el
 * evento `resize`: en el WebView de WebKitGTK ninguno de los dos llega, así que
 * una cuadrícula que dependa de ellos se queda con las columnas del arranque
 * para siempre.
 */
export function useColumnasVisibles(cortes: Corte[]): Ref<number> {
	const columnas = ref(1);
	let observador: ResizeObserver | null = null;

	onMounted(() => {
		const raiz = document.documentElement;

		const recalcular = () => {
			columnas.value = columnasPara(raiz.clientWidth, cortes);
		};

		recalcular();

		// `ResizeObserver` puede no existir en un entorno de pruebas sin DOM
		// completo: sin él queda el ancho del arranque, que es mejor que romper.
		if (typeof ResizeObserver === 'function') {
			observador = new ResizeObserver(recalcular);
			observador.observe(raiz);
		}
	});

	onBeforeUnmount(() => {
		observador?.disconnect();
		observador = null;
	});

	return columnas;
}
