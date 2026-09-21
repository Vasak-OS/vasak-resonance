/**
 * Los dos controles que se anunciaban sin decir de qué eran.
 *
 * El volumen es un `<input type="range">`: sin `<label>` asociado y sin
 * `aria-label`, un lector de pantalla lo lee «control deslizante, 47» y no dice
 * de qué. `aria-valuetext` es lo que hace que ese 47 se oiga como «47 %»; sin
 * él, el número no tiene unidad.
 *
 * El altavoz, además, era un `<svg>` dibujado a mano: el mismo trazo con el
 * volumen en cero que al máximo, y sin seguir al tema del escritorio. Ahora
 * sale del tema y dice en qué tramo está.
 *
 * Y era un `<button>` **sin `@click`** — se podía enfocar, se anunciaba como
 * botón y no hacía nada. Ahora es un `<span>`.
 *
 * El doble de iconos devuelve `simbolo:<nombre>`, así que la ruta dice qué
 * nombre se pidió: es lo que deja comprobar que el tramo elige el icono y no
 * sólo que hay uno.
 */

import { beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import VolumeControl from '@/components/player/VolumeControl.vue';
import { usePlayerStore } from '@/stores/player';

/** El almacén guarda 0..2; lo que se ve es 0..100. */
async function conVolumen(valor: number) {
	const vista = mount(VolumeControl);
	usePlayerStore().volume = valor;
	for (let i = 0; i < 6; i++) {
		await nextTick();
	}
	return vista;
}

beforeEach(() => {
	setActivePinia(createPinia());
});

describe('el volumen', () => {
	test('dice qué regula, y con unidad', async () => {
		const vista = await conVolumen(1);
		const deslizador = vista.get('input[type="range"]');

		expect(deslizador.attributes('aria-label')).toBe('volume.label');
		expect(deslizador.attributes('aria-valuetext')).toBe('50%');
	});

	test('el altavoz dice en qué tramo está', async () => {
		const mudo = await conVolumen(0);
		expect(mudo.get('img').attributes('src')).toBe('simbolo:audio-volume-muted-symbolic');

		const bajo = await conVolumen(0.5);
		expect(bajo.get('img').attributes('src')).toBe('simbolo:audio-volume-low-symbolic');

		const alto = await conVolumen(2);
		expect(alto.get('img').attributes('src')).toBe('simbolo:audio-volume-high-symbolic');
	});

	test('y el altavoz ya no finge ser un botón', async () => {
		// Tenía `:title` y ningún `@click`: enfocable, anunciado como botón, y
		// sin nada detrás. Los únicos botones de esta pieza son ninguno.
		const vista = await conVolumen(1);

		expect(vista.findAll('button')).toHaveLength(0);
	});
});
