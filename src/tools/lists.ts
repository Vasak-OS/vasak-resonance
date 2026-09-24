/**
 * Ayudantes para las listas virtualizadas.
 *
 * El scroller coloca los elementos él, así que hay cosas que no puede leer del
 * CSS y hay que decírselas.
 */

/** Una fila de la cuadrícula, con lo que va adentro. */
export interface GridRow<T> {
	key: string;
	items: T[];
}

/**
 * Parte una colección en filas de `columnas` elementos.
 *
 * Es cómo se virtualiza una cuadrícula acá: **una fila es un elemento del
 * scroller**, y adentro va el `grid` de siempre. La otra manera —el modo
 * cuadrícula del `RecycleScroller`— pide además el ancho de cada columna en
 * píxeles, que hay que medir y mantener al día con el contenedor; así el único
 * número que hay que saber es el alto de la fila.
 *
 * `clave` sale del primer elemento de cada fila, así la fila conserva su
 * identidad mientras no cambie lo que tiene adentro.
 */
export function inRows<T>(
	items: readonly T[],
	columns: number,
	keyOf: (item: T) => string | undefined | null
): GridRow<T>[] {
	const perRow = Math.max(1, Math.floor(columns));
	const rows: GridRow<T>[] = [];

	for (let from = 0; from < items.length; from += perRow) {
		const slice = items.slice(from, from + perRow);
		rows.push({ key: rowKey(keyOf(slice[0]), from), items: slice });
	}

	return rows;
}

/**
 * La clave de una fila, que **no puede faltar**.
 *
 * `DynamicScroller` tira una excepción —`Key is undefined on item`— si un
 * elemento no trae la clave, y Vue se la traga: el componente no dibuja nada y
 * la lista entera queda en blanco, con los datos cargados y sin un solo error a
 * la vista. Un elemento con el dato faltante no puede costar toda la lista.
 *
 * Cuando el primero de la fila no tiene clave se usa su posición. **Las dos
 * salidas van marcadas**, y no sólo la de respaldo: si el respaldo fuera lo
 * único con marca, un elemento cuya clave de verdad fuera justo esa cadena
 * chocaría con la fila sin clave de esa posición, y dos filas con la misma
 * clave son otra vez una lista mal dibujada. Marcando las dos no hay forma de
 * que se crucen.
 */
export function rowKey(own: string | undefined | null, from: number): string {
	return typeof own === 'string' && own !== '' ? `key:${own}` : `position:${from}`;
}
