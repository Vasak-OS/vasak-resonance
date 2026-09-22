import { describe, expect, test } from 'bun:test';
import { claveDeLaFila, enFilas } from '@/tools/listas';

/**
 * Una cuadrícula virtualizada se parte en filas: **una fila es un elemento del
 * scroller**, y adentro va el `grid` de siempre. Que el reparto sea correcto es
 * lo que hace que la cuadrícula se siga viendo igual.
 */

describe('partir una cuadrícula en filas', () => {
	const clave = (n: number) => `e${n}`;

	test('reparte en filas del ancho pedido', () => {
		expect(enFilas([1, 2, 3, 4, 5], 2, clave)).toEqual([
			{ clave: 'e1', elementos: [1, 2] },
			{ clave: 'e3', elementos: [3, 4] },
			{ clave: 'e5', elementos: [5] },
		]);
	});

	test('la última fila puede venir incompleta', () => {
		const filas = enFilas([1, 2, 3, 4, 5, 6, 7], 3, clave);

		expect(filas).toHaveLength(3);
		expect(filas[2].elementos).toEqual([7]);
	});

	test('sin nada, ninguna fila', () => {
		expect(enFilas([], 3, clave)).toEqual([]);
	});

	/**
	 * Cero o menos columnas dejaría un bucle infinito, y el ancho sale de medir
	 * la ventana: mejor una columna que colgar la aplicación.
	 */
	test.each([0, -3, 0.4])('%p columnas se toma como una', (columnas) => {
		expect(enFilas([1, 2], columnas, clave)).toEqual([
			{ clave: 'e1', elementos: [1] },
			{ clave: 'e2', elementos: [2] },
		]);
	});

	test('cada fila conserva su clave mientras no cambie lo que tiene', () => {
		const claves = (columnas: number) =>
			enFilas([1, 2, 3, 4], columnas, clave).map((fila) => fila.clave);

		expect(claves(2)).toEqual(['e1', 'e3']);
		expect(claves(4)).toEqual(['e1']);
	});
});

/**
 * Lo que costó encontrar: `DynamicScroller` **tira una excepción** si un
 * elemento no trae la clave, Vue se la traga, y la lista entera queda en blanco
 * con los datos cargados y sin un error a la vista. Un dato faltante en un
 * elemento no puede costar toda la lista.
 */
describe('la clave de una fila', () => {
	test('cuando el elemento la trae, es la suya', () => {
		expect(claveDeLaFila('abc', 0)).toBe('abc');
	});

	test.each([undefined, null, ''])('cuando falta (%p) se usa la posición', (propia) => {
		expect(claveDeLaFila(propia, 12)).toBe('fila-sin-clave-12');
	});

	test('y dos filas sin clave no chocan entre sí', () => {
		expect(claveDeLaFila(undefined, 0)).not.toBe(claveDeLaFila(undefined, 3));
	});

	/** Con un elemento sin clave, las filas salen igual y todas con clave. */
	test('un elemento sin clave no deja la lista sin filas', () => {
		const sinClave = [{ id: 'a' }, { id: undefined }, { id: 'c' }];
		const filas = enFilas(sinClave, 1, (elemento) => elemento.id);

		expect(filas).toHaveLength(3);
		expect(filas.map((fila) => fila.clave)).toEqual(['a', 'fila-sin-clave-1', 'c']);
		expect(filas.every((fila) => fila.clave !== '' && fila.clave != null)).toBe(true);
	});
});
