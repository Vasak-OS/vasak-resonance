/**
 * El número que el scroller cree y el que el CSS dibuja tienen que ser el mismo.
 *
 * `RecycleScroller` **coloca** las filas a partir de `item-size`: no las mide.
 * Si ese número es menor que lo que la fila ocupa de verdad, las filas se
 * pisan; si es mayor, quedan huecos. Nada falla, nada avisa y el typecheck no
 * tiene cómo enterarse — se ve torcido y listo, que es la clase de error que
 * sobrevive meses.
 *
 * Por eso el alto va en una constante con nombre en cada vista, y acá se
 * comprueba contra las clases de Tailwind que dibujan la fila: el alto propio
 * más el hueco de abajo.
 *
 * Se mira el texto y no se monta porque `happy-dom` no hace maquetado: una
 * prueba que midiera el alto real devolvería cero para todo y pasaría siempre.
 */

import { describe, expect, test } from 'bun:test';
import { fileURLToPath } from 'node:url';

const FUENTE = fileURLToPath(new URL('../src/components/views/', import.meta.url));
const leer = (vista: string) => Bun.file(`${FUENTE}${vista}.vue`).text();

/** Lo que valen en píxeles las clases de hueco que usan estas vistas. */
const HUECO: Record<string, number> = { 'mb-1': 4, 'mb-2': 8, 'mb-3': 12, 'pb-3': 12, 'pb-4': 16 };

/** Las tres que se virtualizaron, con la clase que dibuja el hueco de abajo. */
const VISTAS = [
	{ vista: 'FavoritesView', hueco: 'mb-2' },
	{ vista: 'RadiosView', hueco: 'pb-3' },
	{ vista: 'PlaylistsView', hueco: 'mb-1' },
] as const;

function declarado(texto: string): number | null {
	const m = texto.match(/const ALTO_DE_LA_FILA = (\d+);/);
	return m ? Number(m[1]) : null;
}

function altoDeLaClase(texto: string): number | null {
	const m = texto.match(/\bh-\[(\d+)px\]/);
	return m ? Number(m[1]) : null;
}

describe('el alto de una fila virtualizada', () => {
	test.each(VISTAS.map((v) => [v.vista, v.hueco]))(
		'%s declara el mismo alto que dibuja',
		async (vista, hueco) => {
			const texto = await leer(vista as string);

			const suyo = declarado(texto);
			const dibuja = altoDeLaClase(texto);
			expect(suyo).not.toBeNull();
			expect(dibuja).not.toBeNull();

			// El de la clase incluye el borde y el relleno —`h-` de Tailwind es
			// `height`, y estas filas van con el `box-sizing: border-box` que
			// pone la base de Tailwind—, así que la cuenta es alto + hueco.
			expect(texto).toContain(hueco as string);
			expect(suyo).toBe((dibuja as number) + HUECO[hueco as string]);
		}
	);

	test('las tres dibujan con el scroller y no con un `v-for` suelto', async () => {
		for (const { vista } of VISTAS) {
			const texto = await leer(vista);

			expect(texto).toContain("import { RecycleScroller } from 'vue-virtual-scroller'");
			expect(texto).toMatch(/<RecycleScroller/);
		}
	});

	test('y ninguna recorre su colección entera con un `v-for`', async () => {
		// Es lo que hacían antes: el DOM crecía con la biblioteca, y a 8000
		// filas eran dos segundos y medio de ventana congelada al entrar.
		const recorridos: Record<string, string> = {
			FavoritesView: 'filteredFavoriteEntries',
			RadiosView: 'sortedStations',
			PlaylistsView: 'playlistTracks',
		};

		for (const [vista, coleccion] of Object.entries(recorridos)) {
			const texto = await leer(vista);

			// Cada `v-for` por separado, y se mira sobre qué itera. Armar la
			// expresión con escapes adentro de una plantilla es justo donde se
			// cuela una barra de más y la guardia deja de mirar nada: pasó acá,
			// y se vio comprobándola en negativo.
			const sobreLaColeccion = [...texto.matchAll(/v-for="([^"]*)"/g)]
				.map(([, expresion]) => expresion.trim())
				.filter((expresion) => expresion.endsWith(` in ${coleccion}`));

			expect(sobreLaColeccion).toEqual([]);
		}
	});
});
