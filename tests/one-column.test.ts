/**
 * Ventana angosta: una columna por vez, como una aplicación de teléfono.
 *
 * Antes la barra lateral se apilaba **encima** del contenido por debajo de 768
 * de ventana (`md:`, un punto de corte de la pantalla), y con sus siete
 * secciones y la tapa se comía el alto entero: a 600 y a 360 de ancho el
 * contenido quedaba fuera de la ventana. Ahora la fila mira su propio ancho y,
 * angosta, muestra el contenido; la barra se abre en su lugar con un botón de
 * la barra de la ventana, y elegir una sección la cierra.
 *
 * `happy-dom` no hace maquetado —todo mide cero—, así que el ancho se finge
 * sobre `clientWidth`, que es lo único que lee `useElementWidth`.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import PlaylistsView from '@/components/views/PlaylistsView.vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { isNarrowRow, rowActionsWithLabels, SIDE_BY_SIDE_MIN_WIDTH } from '@/tools/window-layout';
import { contestar, olvidarTodo } from './dobles';

const original = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth');

function pretendWidth(width: number) {
	Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
		configurable: true,
		get: () => width,
	});
}

let view: VueWrapper | null = null;

beforeEach(() => setActivePinia(createPinia()));

afterEach(() => {
	view?.unmount();
	view = null;
	olvidarTodo();
	if (original) Object.defineProperty(HTMLElement.prototype, 'clientWidth', original);
});

describe('el corte de la fila', () => {
	test('es el de `md:` medido en la fila, que es 10 píxeles más angosta que la ventana', () => {
		// El canto del marco y el `p-1` de alrededor: con eso el formato cambia
		// en el mismo ancho de ventana que antes.
		expect(SIDE_BY_SIDE_MIN_WIDTH).toBe(768 - 10);
		expect(isNarrowRow(SIDE_BY_SIDE_MIN_WIDTH - 1)).toBe(true);
		expect(isNarrowRow(SIDE_BY_SIDE_MIN_WIDTH)).toBe(false);
	});

	test('sin medir todavía es ancha, que es el formato de siempre', () => {
		expect(isNarrowRow(0)).toBe(false);
		expect(rowActionsWithLabels(0)).toBe(true);
	});

	test('a 240, 360 y 600 de ventana es una columna; a 1200, dos', () => {
		for (const window of [240, 360, 600]) expect(isNarrowRow(window - 10)).toBe(true);
		expect(isNarrowRow(1200 - 10)).toBe(false);
	});
});

function openWindow(width: number) {
	pretendWidth(width);
	view = mount(WindowAppLayout, {
		global: { stubs: { RouterView: true, ResonanceSidebar: true, NowPlayingTopBar: true } },
	});
	return view;
}

const isShown = (wrapper: VueWrapper, selector: string) =>
	(wrapper.get(selector).element as HTMLElement).style.display !== 'none';

describe('la ventana', () => {
	test('ancha, la barra y el contenido van juntos y no hay botón de biblioteca', async () => {
		const window = openWindow(1190);
		await nextTick();

		expect(isShown(window, 'resonance-sidebar-stub')).toBe(true);
		expect(window.find('button[aria-label="window.showLibrary"]').exists()).toBe(false);
	});

	test('angosta, se ve el contenido y la barra se abre en su lugar', async () => {
		const window = openWindow(590);
		await nextTick();

		expect(isShown(window, 'resonance-sidebar-stub')).toBe(false);

		const toggle = window.get('button[aria-label="window.showLibrary"]');
		await toggle.trigger('click');

		expect(isShown(window, 'resonance-sidebar-stub')).toBe(true);
		// El contenido deja su lugar: una columna por vez.
		const content = window.get('resonance-sidebar-stub').element.nextElementSibling as HTMLElement;
		expect(content.style.display).toBe('none');
		// Y el botón dice que está puesto, y cómo volver.
		const back = window.get('button[aria-label="window.hideLibrary"]');
		expect(back.attributes('aria-pressed')).toBe('true');
	});

	test('la barra ocupa todo el ancho cuando va sola', async () => {
		const window = openWindow(350);
		await nextTick();

		expect(window.get('resonance-sidebar-stub').attributes('fill')).toBe('true');
	});

	test('el nombre de la ventana deja el centro y pasa al título, que se recorta', async () => {
		// Centrado encima de la barra pisaba el botón del mini reproductor a
		// 360 de ancho.
		const narrow = openWindow(350);
		await nextTick();
		expect(narrow.text()).toContain('Resonance');
		expect(narrow.find('.truncate').text()).toContain('Resonance');
	});
});

describe('las listas de reproducción, angostas', () => {
	test('primero la lista de listas; al elegir una, su detalle con volver', async () => {
		contestar('list_playlists_command', [
			{ id: 1, name: 'Para trabajar', created_at: '' },
			{ id: 2, name: 'Domingo', created_at: '' },
		]);
		contestar('list_playlist_tracks_command', []);
		contestar('list_library_tracks', []);
		pretendWidth(330);
		view = mount(PlaylistsView, {
			attachTo: document.body,
			global: { stubs: { PlayerQueuePanel: true, RecycleScroller: true } },
		});
		for (let i = 0; i < 6; i++) await nextTick();

		// `v-show` deja el marcado puesto y lo esconde: lo que cuenta es qué se ve.
		const playlistRows = () => view?.findAll('[role="button"]') ?? [];
		const backButton = () => view?.findAll('button').find((button) => button.text().includes('artists.back'));
		expect(playlistRows()).toHaveLength(2);
		expect(playlistRows()[0]?.isVisible()).toBe(true);
		expect(backButton()?.isVisible() ?? false).toBe(false);

		await playlistRows()[1]?.trigger('click');
		for (let i = 0; i < 6; i++) await nextTick();

		expect(backButton()?.isVisible()).toBe(true);
		expect(playlistRows()[0]?.isVisible()).toBe(false);

		await backButton()?.trigger('click');
		await nextTick();

		expect(playlistRows()[0]?.isVisible()).toBe(true);
		expect(backButton()?.isVisible() ?? false).toBe(false);
	});
});
