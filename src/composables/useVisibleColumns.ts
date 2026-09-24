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
export interface Breakpoint {
	from: number;
	columns: number;
}

/**
 * Las columnas que corresponden a un ancho.
 *
 * Gana el corte de mayor `desde` entre los que el ancho alcanza, que es como se
 * comporta una consulta de medios: la última regla que entra manda. El orden en
 * que vengan los cortes no importa. Sin ninguno alcanzado, una columna, que es
 * lo que hace `grid-cols-1`.
 *
 * **No** gana el que tenga más columnas. Con los cortes de acá los dos criterios
 * dan lo mismo —a más ancho, más columnas—, pero no es lo que dice la regla: con
 * un corte de 3 columnas a los 640 y uno de 2 a los 1280, a 1280 manda el
 * segundo. Elegir por cantidad devolvería 3 y la cuadrícula dibujaría una
 * columna de más sobre filas de dos.
 */
export function columnsFor(width: number, breakpoints: Breakpoint[]): number {
	const winner = breakpoints
		.filter((breakpoint) => width >= breakpoint.from)
		.reduce<Breakpoint | null>(
			(chosen, breakpoint) =>
				chosen === null || breakpoint.from > chosen.from ? breakpoint : chosen,
			null
		);

	return winner?.columns ?? 1;
}

/**
 * Las columnas de ahora, siguiendo al ancho de la ventana.
 *
 * Mira el elemento raíz con un `ResizeObserver` y **no** `matchMedia` ni el
 * evento `resize`: en el WebView de WebKitGTK ninguno de los dos llega, así que
 * una cuadrícula que dependa de ellos se queda con las columnas del arranque
 * para siempre.
 */
export function useVisibleColumns(breakpoints: Breakpoint[]): Ref<number> {
	const columns = ref(1);
	let observer: ResizeObserver | null = null;

	onMounted(() => {
		const root = document.documentElement;

		const recalculate = () => {
			columns.value = columnsFor(root.clientWidth, breakpoints);
		};

		recalculate();

		// `ResizeObserver` puede no existir en un entorno de pruebas sin DOM
		// completo: sin él queda el ancho del arranque, que es mejor que romper.
		if (typeof ResizeObserver === 'function') {
			observer = new ResizeObserver(recalculate);
			observer.observe(root);
		}
	});

	onBeforeUnmount(() => {
		observer?.disconnect();
		observer = null;
	});

	return columns;
}
