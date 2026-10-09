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

const SOURCE = fileURLToPath(new URL('../src/components/views/', import.meta.url));
const read = (view: string) => Bun.file(`${SOURCE}${view}.vue`).text();

/** Lo que valen en píxeles las clases de hueco que usan estas vistas. */
const GAP: Record<string, number> = { 'mb-1': 4, 'mb-2': 8, 'mb-3': 12, 'pb-3': 12, 'pb-4': 16 };

/**
 * Las cuatro de alto fijo, con la clase que dibuja el hueco de abajo y el paso
 * que midió el banco (`apps-shots/bench/shoot-resonance-rows.sh`) en Chrome a
 * 1200 y a 1400 de ventana, antes y después de pasar las filas a `ListRow`:
 * el mismo número, que es lo que se fija acá.
 */
const VIEWS = [
	{ view: 'HomeView', gap: 'mb-2', measured: 92 },
	{ view: 'FavoritesView', gap: 'mb-2', measured: 80 },
	{ view: 'RadiosView', gap: 'pb-3', measured: 128 },
	{ view: 'PlaylistsView', gap: 'mb-1', measured: 58 },
] as const;

function declared(text: string): number | null {
	const m = text.match(/const ROW_HEIGHT = (\d+);/);
	return m ? Number(m[1]) : null;
}

function drawnHeight(text: string): number | null {
	const m = text.match(/\bh-\[(\d+)px\]/);
	return m ? Number(m[1]) : null;
}

describe('el alto de una fila virtualizada', () => {
	test.each(VIEWS.map((v) => [v.view, v.gap, v.measured]))(
		'%s declara el mismo alto que dibuja, y el que midió el banco',
		async (view, gap, measured) => {
			const text = await read(view as string);

			const own = declared(text);
			const drawn = drawnHeight(text);
			expect(own).not.toBeNull();
			expect(drawn).not.toBeNull();

			// El de la clase incluye el borde y el relleno —`h-` de Tailwind es
			// `height`, y estas filas van con el `box-sizing: border-box` que
			// pone la base de Tailwind—, así que la cuenta es alto + hueco.
			expect(text).toContain(gap as string);
			expect(own).toBe((drawn as number) + GAP[gap as string]);
			// Y no cambió al pasar a los componentes de la librería.
			expect(own).toBe(measured as number);
		}
	);

	test('las cuatro dibujan con el scroller y no con un `v-for` suelto', async () => {
		for (const { view } of VIEWS) {
			const text = await read(view);

			expect(text).toMatch(/import \{ RecycleScroller \} from 'vue-virtual-scroller'/);
			expect(text).toMatch(/<RecycleScroller/);
		}
	});

	test('la fila de alto fijo es la que lleva la clase, con `ListRow` adentro a todo el alto', async () => {
		// `ListRow` trae su propio relleno; si se le diera el alto a ella y no
		// al envoltorio, la regla de arriba seguiría pasando mirando otra cosa.
		for (const { view, gap } of VIEWS) {
			if (view === 'RadiosView') continue;
			const text = await read(view);
			const drawn = drawnHeight(text);

			expect(text).toMatch(new RegExp(`class="${gap} h-\\[${drawn}px\\]"`));
			expect(text).toMatch(/<ListRow[^>]*\n?[^>]*class="h-full/);
		}
	});

	test('y ninguna recorre su colección entera con un `v-for`', async () => {
		// Es lo que hacían antes: el DOM crecía con la biblioteca, y a 8000
		// filas eran dos segundos y medio de ventana congelada al entrar.
		const walks: Record<string, string> = {
			FavoritesView: 'filteredFavoriteEntries',
			RadiosView: 'sortedStations',
			PlaylistsView: 'playlistTracks',
		};

		for (const [view, collection] of Object.entries(walks)) {
			const text = await read(view);

			// Cada `v-for` por separado, y se mira sobre qué itera. Armar la
			// expresión con escapes adentro de una plantilla es justo donde se
			// cuela una barra de más y la guardia deja de mirar nada: pasó acá,
			// y se vio comprobándola en negativo.
			const overCollection = [...text.matchAll(/v-for="([^"]*)"/g)]
				.map(([, expression]) => expression.trim())
				.filter((expression) => expression.endsWith(` in ${collection}`));

			expect(overCollection).toEqual([]);
		}
	});
});

describe('la tarjeta de álbum, que se mide en vez de declararse', () => {
	/**
	 * `AlbumsView` usa `DynamicScroller`: mide cada fila, así que un alto
	 * distinto no la rompe. Pero cambiarlo es cambiar el formato de la
	 * pantalla. Medido en el banco antes y después: 518 px a 1200 de ventana y
	 * 534 a 1400. Lo que lo sostiene son estas tres medidas, que eran las del
	 * marcado a mano: la franja de la tapa, la fila de botones con texto y la de
	 * cada canción de la vista previa.
	 */
	test('las medidas que sostienen el alto de la tarjeta siguen puestas', async () => {
		const text = await read('AlbumsView');

		expect(text).toContain('class="mb-3 flex h-44 justify-center"');
		expect(text).toContain('auto-rows-[minmax(2.125rem,auto)]');
		expect(text).toContain('min-h-[2.375rem]');
		expect(text).toContain('const MIN_CARD_HEIGHT = 532;');
	});
});
