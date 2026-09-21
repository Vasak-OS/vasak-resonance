/**
 * La décima copia del composable del icono, y lo que se fue con ella.
 *
 * `useReactiveIcon()` estaba en nueve aplicaciones además de ésta, todas
 * iguales, todas nacidas del mismo apartado del README de la plantilla. No
 * fallaba nunca: pedía el icono al montar y escuchaba el cambio de tema, uno
 * por instancia, contra el oyente único que la librería tiene para todo el
 * escritorio. Treinta llamadas en nueve archivos acá.
 *
 * Que el archivo no esté no alcanza como prueba: lo que hay que vigilar es que
 * nadie lo vuelva a escribir con otro nombre. Por eso además se comprueba que
 * ningún `.vue` resuelva iconos por su cuenta —`getIconSource`,
 * `getSymbolSource` o un oyente de `vicons:theme-changed`—, que es en lo que
 * consistía la copia.
 */

import { describe, expect, test } from 'bun:test';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const RAIZ = join(import.meta.dir, '..');

function todosLosFuentes(directorio: string, acumulado: string[] = []): string[] {
	for (const entrada of readdirSync(directorio)) {
		const ruta = join(directorio, entrada);
		if (statSync(ruta).isDirectory()) {
			todosLosFuentes(ruta, acumulado);
		} else if (ruta.endsWith('.vue') || ruta.endsWith('.ts')) {
			acumulado.push(ruta);
		}
	}
	return acumulado;
}

const FUENTES = todosLosFuentes(join(RAIZ, 'src'));

describe('el composable del icono', () => {
	test('ya no existe', async () => {
		expect(await Bun.file(join(RAIZ, 'src/composables/useReactiveIcon.ts')).exists()).toBe(false);
	});

	test('y nadie lo llama', () => {
		const culpables = FUENTES.filter((ruta) =>
			/useReactiveIcon\s*\(/.test(readFileSync(ruta, 'utf8'))
		);

		expect(culpables).toEqual([]);
	});
});

/**
 * `src/main.ts` queda afuera, y es el único.
 *
 * El menú contextual del escritorio no dibuja con Vue: pide una **función** que
 * le resuelva el nombre a una ruta, porque lo pinta el complemento fuera de
 * esta ventana. `ThemeIcon` no sirve ahí, así que `getIconSource` es la
 * respuesta correcta y no una copia.
 */
const EXCEPCIONES = [join(RAIZ, 'src/main.ts')];

describe('nadie resuelve iconos por su cuenta', () => {
	// Es la forma de la copia, no su nombre: pedirle la ruta al complemento y
	// volver a pedírsela cuando cambia el tema. Eso lo hace `ThemeIcon` una sola
	// vez para toda la ventana, con un oyente compartido.
	for (const patron of [/getIconSource/, /getSymbolSource/, /vicons:theme-changed/]) {
		test(String(patron), () => {
			const culpables = FUENTES.filter(
				(ruta) => !EXCEPCIONES.includes(ruta) && patron.test(readFileSync(ruta, 'utf8'))
			);

			expect(culpables).toEqual([]);
		});
	}
});
