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
export function columnasPara(ancho: number, cortes: Corte[]): number {
	const manda = cortes
		.filter((corte) => ancho >= corte.desde)
		.reduce<Corte | null>(
			(elegido, corte) => (elegido === null || corte.desde > elegido.desde ? corte : elegido),
			null
		);

	return manda?.columnas ?? 1;
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
