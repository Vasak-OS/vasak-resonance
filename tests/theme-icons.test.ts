/**
 * Los iconos del reproductor siguen al tema, y la recarga va por el planificador.
 *
 * Resonance no resuelve ningún icono: los pide por nombre a `ThemeIcon`, y son
 * treinta y dos. Lo que se comprueba acá es que la recarga llegue por el
 * planificador de la librería y no en el acto.
 *
 * Importa porque hasta este cambio no era así, y nada lo decía: el manifiesto
 * pedía `^1.2.0`, que admite la 1.4.0, pero `bun.lock` había quedado en la
 * 1.2.0 y se empaquetaba ésa. Un rango que ya se satisface no mueve el candado,
 * y el CI decía «al día» porque sólo miraba el rango.
 */

import { afterEach, beforeEach, describe, expect, jest, test } from 'bun:test';
import { olvidarLosIconosDelTema } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import TransportButton from '@/components/player/transport/TransportButton.vue';
import { emit, olvidarTodo, setThemeIcon } from './dobles';

/**
 * Deja que terminen las promesas encadenadas del pedido del icono.
 *
 * Sólo microtareas: con el reloj detenido, un `setTimeout(0)` no vuelve nunca.
 */
async function settle(rounds = 8) {
	for (let i = 0; i < rounds; i++) {
		await nextTick();
		await Promise.resolve();
	}
}

/**
 * Adelanta el reloj hasta pasada la espera del planificador, y asienta.
 *
 * Con temporizadores falsos y no con una espera de verdad. Las dos cosas que
 * hay que comprobar acá se pelean: que la recarga **todavía no** pasó justo
 * después del evento, y que **sí** pasa un poco más tarde. Con el reloj real la
 * primera falla de a ratos —si la máquina se demora, los 100 ms se cumplen
 * antes de la aserción— y con sólo un `nextTick` la segunda se vuelve vacía:
 * sin planificador la recarga tampoco llega a verse, así que la prueba pasaría
 * con la librería vieja.
 *
 * Se avanza en pasos porque el planificador `await`ea entre tandas y las
 * microtareas tienen que poder correr en el medio.
 */
async function advancePastReload() {
	for (let i = 0; i < 8; i++) {
		jest.advanceTimersByTime(40);
		await settle(2);
	}
}

let mounted: VueWrapper | null = null;

function mountButton() {
	mounted = mount(TransportButton, {
		props: { label: 'Reproducir', icon: 'media-playback-start' },
	});
	return mounted;
}

beforeEach(() => {
	jest.useFakeTimers();
	olvidarTodo();
	// La memoria de la librería vive en su módulo y sobrevive entre archivos de
	// prueba: sin vaciarla, esto ve el icono que dejó otra.
	olvidarLosIconosDelTema();
});

afterEach(() => {
	mounted?.unmount();
	mounted = null;
	olvidarLosIconosDelTema();
	jest.useRealTimers();
});

describe('el reproductor dibuja sus iconos con el tema', () => {
	test('el botón de transporte lo pide por nombre y monocromo', async () => {
		setThemeIcon('media-playback-start', 'data:image/svg+xml,play-claro');

		const boton = mountButton();
		await settle();

		expect(boton.get('img').attributes('src')).toBe('data:image/svg+xml,play-claro');
	});

	test('la recarga se agenda, no pasa en el acto', async () => {
		// Es lo que separa la 1.2.0 —la que se venía empaquetando— de la 1.4.0:
		// sin planificador el dibujo nuevo ya estaría acá. Con treinta y dos
		// iconos a la vez, ese barrido de más es la diferencia que se ve.
		setThemeIcon('media-playback-start', 'data:image/svg+xml,play-claro');

		const boton = mountButton();
		await settle();

		setThemeIcon('media-playback-start', 'data:image/svg+xml,play-oscuro');
		await emit('vicons:theme-changed');
		await settle();

		expect(boton.get('img').attributes('src')).not.toBe('data:image/svg+xml,play-oscuro');

		await advancePastReload();
		expect(boton.get('img').attributes('src')).toBe('data:image/svg+xml,play-oscuro');
	});
});
