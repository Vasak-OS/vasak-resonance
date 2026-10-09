/**
 * Cuándo la ventana tiene lugar para la barra lateral y el contenido a la vez.
 *
 * Antes lo decía `md:`, un punto de corte **de la pantalla**: 768 píxeles de
 * ventana. La fila donde viven las dos columnas es 10 píxeles más angosta que
 * la ventana —el canto del marco y el `p-1` de alrededor—, así que medido en
 * la fila el mismo corte es 758. Con eso el formato cambia exactamente en el
 * mismo ancho que antes, pero lo decide el lugar que tiene la ventana y no el
 * monitor: en WebKitGTK ni `matchMedia` ni `resize` avisan.
 */
export const SIDE_BY_SIDE_MIN_WIDTH = 758;

/**
 * Si la fila es angosta: una columna por vez, como en un teléfono.
 *
 * Un ancho cero es «todavía no se midió» —la primera pasada, o un entorno sin
 * maquetado— y se toma como ancho, que es el formato de siempre.
 */
export function isNarrowRow(width: number): boolean {
	return width > 0 && width < SIDE_BY_SIDE_MIN_WIDTH;
}

/**
 * Desde qué ancho una fila de canción lleva sus botones con texto.
 *
 * Por debajo, «Reproducir» y «Favorito» quedan como iconos con nombre: los dos
 * botones con texto ocupan unos 220 píxeles, y en una ventana de teléfono eso
 * es la fila entera.
 */
export const LABELED_ROW_ACTIONS_MIN_WIDTH = 480;

export function rowActionsWithLabels(width: number): boolean {
	return width === 0 || width >= LABELED_ROW_ACTIONS_MIN_WIDTH;
}
