/**
 * La onda de abajo entra en el ancho que tiene.
 *
 * Son 110 barras con un hueco de 2 píxeles entre cada una: en una ventana de
 * 240 los huecos solos ocupaban más que la tira, las barras quedaban en cero y
 * la onda desaparecía sin que nada lo dijera. Ahora se dibujan las que entran
 * —cuatro píxeles por barra como mínimo— y, con lugar, las de siempre.
 *
 * `happy-dom` no hace maquetado, así que el ancho se finge sobre
 * `clientWidth`, que es lo que lee `useElementWidth`.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import PlaybackWaves from '@/components/player/PlaybackWaves.vue';

const original = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth');
let view: VueWrapper | null = null;

async function barsAt(width: number): Promise<number> {
	Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
		configurable: true,
		get: () => width,
	});
	view = mount(PlaybackWaves);
	await nextTick();
	return view.findAll('span.flex-1').length;
}

beforeEach(() => setActivePinia(createPinia()));

afterEach(() => {
	view?.unmount();
	view = null;
	if (original) Object.defineProperty(HTMLElement.prototype, 'clientWidth', original);
});

describe('la onda de la barra', () => {
	test('con lugar, son las 110 de siempre', async () => {
		expect(await barsAt(860)).toBe(110);
	});

	test('angosta, las que entran a cuatro píxeles cada una', async () => {
		expect(await barsAt(200)).toBe(50);
	});

	test('y nunca menos de ocho, para que siga siendo una onda', async () => {
		expect(await barsAt(10)).toBe(8);
	});

	test('sin medir todavía, las de siempre', async () => {
		expect(await barsAt(0)).toBe(110);
	});
});
