import { describe, expect, test } from 'bun:test';
import { type Corte, columnasPara } from '@/composables/useColumnasVisibles';

/**
 * El `RecycleScroller` coloca los elementos él, así que hay que decirle cuántas
 * columnas hay. Estos son los cortes que traían las clases de Tailwind, y el
 * punto de estas pruebas es que la cuadrícula se siga viendo igual.
 */

/** Los de Álbumes: `grid sm:grid-cols-2 xl:grid-cols-3`. */
const DISCOS: Corte[] = [
	{ desde: 640, columnas: 2 },
	{ desde: 1280, columnas: 3 },
];

/** Los de Radios: `grid-cols-1 md:grid-cols-2 lg:grid-cols-3`. */
const EMISORAS: Corte[] = [
	{ desde: 768, columnas: 2 },
	{ desde: 1024, columnas: 3 },
];

describe('cuántas columnas dibuja la cuadrícula', () => {
	test('sin llegar a ningún corte, una sola', () => {
		expect(columnasPara(0, DISCOS)).toBe(1);
		expect(columnasPara(639, DISCOS)).toBe(1);
		expect(columnasPara(767, EMISORAS)).toBe(1);
	});

	test('justo en el corte ya cuenta', () => {
		expect(columnasPara(640, DISCOS)).toBe(2);
		expect(columnasPara(768, EMISORAS)).toBe(2);
		expect(columnasPara(1280, DISCOS)).toBe(3);
		expect(columnasPara(1024, EMISORAS)).toBe(3);
	});

	test('entre dos cortes manda el de abajo', () => {
		expect(columnasPara(1279, DISCOS)).toBe(2);
		expect(columnasPara(1023, EMISORAS)).toBe(2);
	});

	test('y más ancho que el último corte no suma columnas', () => {
		expect(columnasPara(4000, DISCOS)).toBe(3);
		expect(columnasPara(4000, EMISORAS)).toBe(3);
	});

	/** El orden en que vengan los cortes no puede cambiar el resultado. */
	test('los cortes desordenados dan lo mismo', () => {
		const alReves = [...DISCOS].reverse();

		for (const ancho of [0, 639, 640, 1279, 1280, 3000]) {
			expect(columnasPara(ancho, alReves)).toBe(columnasPara(ancho, DISCOS));
		}
	});

	test('sin cortes, siempre una', () => {
		expect(columnasPara(5000, [])).toBe(1);
	});
});
