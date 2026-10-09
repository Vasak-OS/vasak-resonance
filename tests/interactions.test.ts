import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { SideButton } from '@vasakgroup/vue-libvasak';
import { mount } from '@vue/test-utils';

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
const ROOT = join(import.meta.dir, '..');
const read = (...parts: string[]) => readFileSync(join(ROOT, ...parts), 'utf8');

const SOURCES = [
	'src/components/layout/ResonanceSidebar.vue',
	'src/components/player/PlaybackWaves.vue',
	'src/components/player/PlayerQueuePanel.vue',
	'src/components/views/HomeView.vue',
	'src/layouts/WindowAppLayout.vue',
];

describe('ninguna transición pide «todas»', () => {
	for (const file of SOURCES) {
		test(file, () => {
			expect(read(file)).not.toContain('transition-all');
		});
	}
});

/**
 * El botón de la barra no se dibuja acá: es `SideButton` de la librería.
 *
 * Por eso esto se mira en el botón **montado** y no en el texto del `.vue`: la
 * clase vive en la dependencia, así que un `transition-all` que volviera allá
 * pasaría por al lado de un `expect(SIDEBAR).toContain(...)` sin despeinarlo.
 *
 * Desde la 2.x de la librería el botón ya no se hunde al apretarlo: la forma de
 * Once UI marca el estado con el velo (`ui-hover`, `ui-pressed`) y no con
 * escalas, que además la guardia del diseño prohíbe. Lo que se sigue fijando
 * es que la transición nombre lo que cambia y no «todas».
 */
describe('el botón de la barra lateral', () => {
	test('transiciona los colores, no todo, y no se escala', () => {
		const button = mount(SideButton, { props: { label: 'Inicio' } });

		const classes = button.get('button').classes();
		expect(classes).toContain('transition-colors');
		expect(classes).not.toContain('transition-all');
		expect(classes.some((name) => /scale-/.test(name))).toBe(false);
	});

	test('y la barra lo usa en vez de dibujar el suyo', () => {
		// Sin esto, la de arriba comprueba la librería y nadie comprueba que la
		// aplicación la use.
		const sidebar = read('src/components/layout/ResonanceSidebar.vue');

		expect(sidebar).toContain('SideButton');
		expect(sidebar).not.toContain('<button');
	});
});

describe('los paneles', () => {
	// El aviso de la ventana ya no tiene transición propia: es `ToastArea`.
	for (const file of ['src/components/player/PlayerQueuePanel.vue']) {
		test(`${file} anima la propiedad que de verdad cambia`, () => {
			const source = read(file);

			// Mismo motivo que la barra lateral: lo que mueven los `translate-*`
			// en Tailwind 4 es `translate`, no `transform`.
			expect(source).toContain('transition-[opacity,translate]');
			expect(source).not.toContain('transition-[opacity,transform]');
		});
	}
});

describe('la canción que suena', () => {
	const CSS = read('src/assets/main.css');
	const HOME = read('src/components/views/HomeView.vue');

	test('se marca en la fila de la lista', () => {
		expect(HOME).toContain("'track-playing': track.path === playerStore.currentPath");
	});

	test('late sólo mientras suena de verdad', () => {
		// En pausa queda encendida y quieta: una animación infinita mantiene al
		// compositor despierto para siempre.
		expect(HOME).toContain('playerStore.isPlaying');
		expect(CSS).toContain('.track-playing--active::after');
		expect(CSS).toContain('animation: track-pulse');
	});

	test('anima la opacidad y no la sombra', () => {
		const pulse = CSS.slice(CSS.indexOf('@keyframes track-pulse'));
		const body = pulse.slice(0, pulse.indexOf('}\n}') + 3);

		expect(body).toContain('opacity');
		expect(body).not.toContain('box-shadow');
	});

	test('el resplandor no se come los clics de la fila', () => {
		const pseudo = CSS.slice(CSS.indexOf('.track-playing::after'));

		expect(pseudo.slice(0, pseudo.indexOf('}'))).toContain('pointer-events: none');
	});
});

describe('quien pidió menos movimiento', () => {
	test('sigue teniendo el bloque que apaga lo infinito', () => {
		const CSS = read('src/assets/main.css');

		expect(CSS).toContain('@media (prefers-reduced-motion: reduce)');
		expect(CSS).toContain('animation-iteration-count: 1 !important');
		// Y el bloque tiene que ir después de lo que apaga, o no le gana.
		expect(CSS.indexOf('@media (prefers-reduced-motion: reduce)')).toBeGreaterThan(
			CSS.indexOf('@keyframes track-pulse')
		);
	});
});
