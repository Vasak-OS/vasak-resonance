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
	claveDe: (elemento: T) => string
): FilaDeCuadricula<T>[] {
	const porFila = Math.max(1, Math.floor(columnas));
	const filas: FilaDeCuadricula<T>[] = [];

	for (let desde = 0; desde < elementos.length; desde += porFila) {
		const dela = elementos.slice(desde, desde + porFila);
		filas.push({ clave: claveDe(dela[0]), elementos: dela });
	}

	return filas;
}
