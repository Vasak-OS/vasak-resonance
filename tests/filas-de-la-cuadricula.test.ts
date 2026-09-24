import { describe, expect, test } from 'bun:test';
import { inRows, rowKey } from '@/tools/lists';

/**
 * Una cuadrícula virtualizada se parte en filas: **una fila es un elemento del
 * scroller**, y adentro va el `grid` de siempre. Que el reparto sea correcto es
 * lo que hace que la cuadrícula se siga viendo igual.
 */

describe('partir una cuadrícula en filas', () => {
	const clave = (n: number) => `e${n}`;

	test('reparte en filas del ancho pedido', () => {
		expect(inRows([1, 2, 3, 4, 5], 2, clave)).toEqual([
			{ key: 'key:e1', items: [1, 2] },
			{ key: 'key:e3', items: [3, 4] },
			{ key: 'key:e5', items: [5] },
		]);
	});

	test('la última fila puede venir incompleta', () => {
		const filas = inRows([1, 2, 3, 4, 5, 6, 7], 3, clave);

		expect(filas).toHaveLength(3);
		expect(filas[2].items).toEqual([7]);
	});

	test('sin nada, ninguna fila', () => {
		expect(inRows([], 3, clave)).toEqual([]);
	});

	/**
	 * Cero o menos columnas dejaría un bucle infinito, y el ancho sale de medir
	 * la ventana: mejor una columna que colgar la aplicación.
	 */
	test.each([0, -3, 0.4])('%p columnas se toma como una', (columnas) => {
		expect(inRows([1, 2], columnas, clave)).toEqual([
			{ key: 'key:e1', items: [1] },
			{ key: 'key:e2', items: [2] },
		]);
	});

	test('cada fila conserva su clave mientras no cambie lo que tiene', () => {
		const claves = (columnas: number) =>
			inRows([1, 2, 3, 4], columnas, clave).map((fila) => fila.key);

		expect(claves(2)).toEqual(['key:e1', 'key:e3']);
		expect(claves(4)).toEqual(['key:e1']);
	});
});

/**
 * Lo que costó encontrar: `DynamicScroller` **tira una excepción** si un
 * elemento no trae la clave, Vue se la traga, y la lista entera queda en blanco
 * con los datos cargados y sin un error a la vista. Un dato faltante en un
 * elemento no puede costar toda la lista.
 */
describe('la clave de una fila', () => {
	test('cuando el elemento la trae, se usa la suya', () => {
		expect(rowKey('abc', 0)).toBe('key:abc');
	});

	test.each([undefined, null, ''])('cuando falta (%p) se usa la posición', (propia) => {
		expect(rowKey(propia, 12)).toBe('position:12');
	});

	test('y dos filas sin clave no chocan entre sí', () => {
		expect(rowKey(undefined, 0)).not.toBe(rowKey(undefined, 3));
	});

	/**
	 * Y una clave de verdad no puede chocar con una de respaldo, aunque el
	 * elemento traiga justo esa cadena: por eso van marcadas **las dos**.
	 */
	test('una clave con pinta de respaldo tampoco choca', () => {
		expect(rowKey('position:0', 7)).not.toBe(rowKey(undefined, 0));
		expect(rowKey('position:5', 5)).not.toBe(rowKey(undefined, 5));
	});

	/** Con un elemento sin clave, las filas salen igual y todas con clave. */
	test('un elemento sin clave no deja la lista sin filas', () => {
		const sinClave = [{ id: 'a' }, { id: undefined }, { id: 'c' }];
		const filas = inRows(sinClave, 1, (elemento) => elemento.id);

		expect(filas).toHaveLength(3);
		expect(filas.map((fila) => fila.key)).toEqual(['key:a', 'position:1', 'key:c']);
		expect(filas.every((fila) => fila.key !== '' && fila.key != null)).toBe(true);
	});
});
