/**
 * Ayudantes para las listas virtualizadas.
 *
 * El scroller coloca los elementos él, así que hay cosas que no puede leer del
 * CSS y hay que decírselas.
 */

/** Una fila de la cuadrícula, con lo que va adentro. */
export interface FilaDeCuadricula<T> {
	clave: string;
	elementos: T[];
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
export function enFilas<T>(
	elementos: readonly T[],
	columnas: number,
	claveDe: (elemento: T) => string | undefined | null
): FilaDeCuadricula<T>[] {
	const porFila = Math.max(1, Math.floor(columnas));
	const filas: FilaDeCuadricula<T>[] = [];

	for (let desde = 0; desde < elementos.length; desde += porFila) {
		const dela = elementos.slice(desde, desde + porFila);
		filas.push({ clave: claveDeLaFila(claveDe(dela[0]), desde), elementos: dela });
	}

	return filas;
}

/**
 * La clave de una fila, que **no puede faltar**.
 *
 * `DynamicScroller` tira una excepción —`Key is undefined on item`— si un
 * elemento no trae la clave, y Vue se la traga: el componente no dibuja nada y
 * la lista entera queda en blanco, con los datos cargados y sin un solo error a
 * la vista. Un elemento con el dato faltante no puede costar toda la lista.
 *
 * Cuando el primero de la fila no tiene clave se usa su posición, que es única
 * entre filas y con un prefijo que no puede chocar con una clave de verdad.
 */
export function claveDeLaFila(propia: string | undefined | null, desde: number): string {
	return typeof propia === 'string' && propia !== '' ? propia : `fila-sin-clave-${desde}`;
}
