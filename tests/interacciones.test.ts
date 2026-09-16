import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El pulido visual del reproductor, con una condición: que no cueste cuadros.
 *
 * Lo que se fija acá no es el gusto —los colores y las duraciones se cambian
 * cuando se quiera— sino las dos reglas que hacen que este pulido sea gratis:
 * que ninguna transición diga «todas las propiedades», y que el resplandor de
 * la canción que suena anime `opacity` y nunca `box-shadow`.
 *
 * `transition-all` obliga al navegador a mirar cada propiedad animable del
 * elemento en cada cambio, incluidas las que nadie toca. Es la forma más fácil
 * de pagar layout sin querer: alcanza con que alguien agregue un `height` a la
 * clase para que empiece a interpolarse.
 */
const RAIZ = join(import.meta.dir, '..');
const leer = (...partes: string[]) => readFileSync(join(RAIZ, ...partes), 'utf8');

const FUENTES = [
	'src/components/layout/ResonanceSidebar.vue',
	'src/components/player/PlaybackWaves.vue',
	'src/components/player/PlayerQueuePanel.vue',
	'src/components/views/HomeView.vue',
	'src/layouts/WindowAppLayout.vue',
];

describe('ninguna transición pide «todas»', () => {
	for (const archivo of FUENTES) {
		test(archivo, () => {
			expect(leer(archivo)).not.toContain('transition-all');
		});
	}
});

describe('la barra lateral', () => {
	const SIDEBAR = leer('src/components/layout/ResonanceSidebar.vue');

	test('transiciona color y la escala, no el resto', () => {
		// `scale` y no `transform`: en Tailwind 4 las utilidades `scale-*` y
		// `translate-*` escriben las propiedades nativas `scale` y `translate`,
		// así que nombrar `transform` en la lista deja el movimiento sin animar
		// —sin error, simplemente no transiciona—. Comprobado en el CSS que sale
		// del build: `.-translate-y-2` emite `translate: …`.
		expect(SIDEBAR).toContain('transition-[color,background-color,border-color,scale]');
	});

	test('se hunde al hacer clic, con transform y no con tamaño', () => {
		// `scale` es composición; cambiar el tamaño sería layout de toda la lista.
		expect(SIDEBAR).toContain('active:scale-[0.98]');
	});
});

describe('los paneles', () => {
	for (const archivo of [
		'src/layouts/WindowAppLayout.vue',
		'src/components/player/PlayerQueuePanel.vue',
	]) {
		test(`${archivo} anima la propiedad que de verdad cambia`, () => {
			const fuente = leer(archivo);

			// Mismo motivo que la barra lateral: lo que mueven los `translate-*`
			// en Tailwind 4 es `translate`, no `transform`.
			expect(fuente).toContain('transition-[opacity,translate]');
			expect(fuente).not.toContain('transition-[opacity,transform]');
		});
	}
});

describe('la canción que suena', () => {
	const CSS = leer('src/assets/main.css');
	const HOME = leer('src/components/views/HomeView.vue');

	test('se marca en la fila de la lista', () => {
		expect(HOME).toContain("'pista-sonando': track.path === playerStore.currentPath");
	});

	test('late sólo mientras suena de verdad', () => {
		// En pausa queda encendida y quieta: una animación infinita mantiene al
		// compositor despierto para siempre.
		expect(HOME).toContain('playerStore.isPlaying');
		expect(CSS).toContain('.pista-sonando--activa::after');
		expect(CSS).toContain('animation: latido-de-pista');
	});

	test('anima la opacidad y no la sombra', () => {
		const latido = CSS.slice(CSS.indexOf('@keyframes latido-de-pista'));
		const cuerpo = latido.slice(0, latido.indexOf('}\n}') + 3);

		expect(cuerpo).toContain('opacity');
		expect(cuerpo).not.toContain('box-shadow');
	});

	test('el resplandor no se come los clics de la fila', () => {
		const pseudo = CSS.slice(CSS.indexOf('.pista-sonando::after'));

		expect(pseudo.slice(0, pseudo.indexOf('}'))).toContain('pointer-events: none');
	});
});

describe('quien pidió menos movimiento', () => {
	test('sigue teniendo el bloque que apaga lo infinito', () => {
		const CSS = leer('src/assets/main.css');

		expect(CSS).toContain('@media (prefers-reduced-motion: reduce)');
		expect(CSS).toContain('animation-iteration-count: 1 !important');
		// Y el bloque tiene que ir después de lo que apaga, o no le gana.
		expect(CSS.indexOf('@media (prefers-reduced-motion: reduce)')).toBeGreaterThan(
			CSS.indexOf('@keyframes latido-de-pista')
		);
	});
});
