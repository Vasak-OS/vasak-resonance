import { describe, expect, test } from 'bun:test';
import { type Breakpoint, columnsFor } from '@/composables/useVisibleColumns';

/**
 * El `RecycleScroller` coloca los elementos él, así que hay que decirle cuántas
 * columnas hay. Estos son los cortes que traían las clases de Tailwind, y el
 * punto de estas pruebas es que la cuadrícula se siga viendo igual.
 */

/** Los de Álbumes: `grid sm:grid-cols-2 xl:grid-cols-3`. */
const DISCOS: Breakpoint[] = [
	{ from: 640, columns: 2 },
	{ from: 1280, columns: 3 },
];

/** Los de Radios: `grid-cols-1 md:grid-cols-2 lg:grid-cols-3`. */
const EMISORAS: Breakpoint[] = [
	{ from: 768, columns: 2 },
	{ from: 1024, columns: 3 },
];

describe('cuántas columnas dibuja la cuadrícula', () => {
	test('sin llegar a ningún corte, una sola', () => {
		expect(columnsFor(0, DISCOS)).toBe(1);
		expect(columnsFor(639, DISCOS)).toBe(1);
		expect(columnsFor(767, EMISORAS)).toBe(1);
	});

	test('justo en el corte ya cuenta', () => {
		expect(columnsFor(640, DISCOS)).toBe(2);
		expect(columnsFor(768, EMISORAS)).toBe(2);
		expect(columnsFor(1280, DISCOS)).toBe(3);
		expect(columnsFor(1024, EMISORAS)).toBe(3);
	});

	test('entre dos cortes manda el de abajo', () => {
		expect(columnsFor(1279, DISCOS)).toBe(2);
		expect(columnsFor(1023, EMISORAS)).toBe(2);
	});

	test('y más ancho que el último corte no suma columnas', () => {
		expect(columnsFor(4000, DISCOS)).toBe(3);
		expect(columnsFor(4000, EMISORAS)).toBe(3);
	});

	/** El orden en que vengan los cortes no puede cambiar el resultado. */
	test('los cortes desordenados dan lo mismo', () => {
		const alReves = [...DISCOS].reverse();

		for (const ancho of [0, 639, 640, 1279, 1280, 3000]) {
			expect(columnsFor(ancho, alReves)).toBe(columnsFor(ancho, DISCOS));
		}
	});

	/**
	 * Manda el corte de mayor `desde`, no el de más columnas.
	 *
	 * Con los cortes que usa la aplicación los dos criterios dan lo mismo —a más
	 * ancho, más columnas—, así que esto sólo se ve con una tabla que baje. Es
	 * la regla de una consulta de medios: la última que entra manda.
	 */
	test('con una tabla que baja, manda el corte de más arriba', () => {
		const queBaja: Breakpoint[] = [
			{ from: 640, columns: 3 },
			{ from: 1280, columns: 2 },
		];

		expect(columnsFor(639, queBaja)).toBe(1);
		expect(columnsFor(640, queBaja)).toBe(3);
		expect(columnsFor(1279, queBaja)).toBe(3);
		expect(columnsFor(1280, queBaja)).toBe(2);
		expect(columnsFor(4000, queBaja)).toBe(2);
	});

	/** Y tampoco cambia si vienen desordenados. */
	test('la tabla que baja, desordenada, da lo mismo', () => {
		const queBaja: Breakpoint[] = [
			{ from: 1280, columns: 2 },
			{ from: 640, columns: 3 },
		];

		expect(columnsFor(1280, queBaja)).toBe(2);
		expect(columnsFor(700, queBaja)).toBe(3);
	});

	test('sin cortes, siempre una', () => {
		expect(columnsFor(5000, [])).toBe(1);
	});
});
