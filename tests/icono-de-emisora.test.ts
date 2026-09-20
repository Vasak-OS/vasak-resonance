/**
 * El icono de una emisora que no carga.
 *
 * Acá había un `onerror="this.style.display='none'"`, que es un manejador **en
 * línea**: la política de contenido de esta ventana no permite `script-src`
 * inline, así que el navegador nunca lo ejecutaba. Un favicon roto dejaba
 * puesto el dibujo de imagen rota, que es justo lo que se quería evitar.
 *
 * Lo destapó `strictTemplates`: `onerror` no es un atributo de `<img>` en los
 * tipos de Vue —el evento es `@error`—, así que dejó de pasar en silencio.
 */

import { afterEach, describe, expect, test } from 'bun:test';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import RadiosView from '@/components/views/RadiosView.vue';
import { contestar, olvidarTodo } from './dobles';

let vista: VueWrapper | null = null;

/** Deja que terminen las promesas del montaje. */
async function asentar(vueltas = 8) {
	for (let i = 0; i < vueltas; i++) {
		await Promise.resolve();
		await new Promise((sigue) => setTimeout(sigue, 0));
	}
}

afterEach(() => {
	vista?.unmount();
	vista = null;
	olvidarTodo();
	// El servicio guarda la lista en `localStorage` durante una hora, y una
	// prueba que la deje puesta le cambia el punto de partida a la siguiente.
	localStorage.clear();
});

describe('una emisora con favicon', () => {
	test('cuando el icono falla, se muestra el de la aplicación', async () => {
		setActivePinia(createPinia());
		contestar('fetch_radio_stations', [
			{ uuid: 'una', name: 'Radio Uno', url: 'http://x', favicon: 'http://no-existe/f.png' },
		]);
		vista = mount(RadiosView);
		await asentar();

		const icono = vista.find('img[alt="Radio Uno"]');
		expect(icono.exists()).toBe(true);

		await icono.trigger('error');

		// El `<img>` de la emisora se va, y queda el bloque con el icono de la
		// aplicación —el mismo que se ve cuando la emisora no trae ninguno—.
		expect(vista.find('img[alt="Radio Uno"]').exists()).toBe(false);
	});

	test('y con la lista nueva se vuelve a intentar', async () => {
		// La marca de «este icono no carga» dura lo que dura la lista. Si la
		// emisora arregla su icono y una recarga lo trae bien, el `<img>` tiene
		// que volver: antes el UUID quedaba marcado hasta cerrar la ventana y
		// esa emisora se quedaba con el icono de la aplicación para siempre. Lo
		// marcó la revisión.
		setActivePinia(createPinia());
		contestar('fetch_radio_stations', [
			{ uuid: 'una', name: 'Radio Uno', url: 'http://x', favicon: 'http://no-existe/f.png' },
		]);
		vista = mount(RadiosView);
		await asentar();
		await vista.get('img[alt="Radio Uno"]').trigger('error');
		expect(vista.find('img[alt="Radio Uno"]').exists()).toBe(false);

		// La misma emisora, con el icono arreglado, tras tocar otra etiqueta.
		contestar('fetch_radio_stations', [
			{ uuid: 'una', name: 'Radio Uno', url: 'http://x', favicon: 'http://si-existe/f.png' },
		]);
		const otraEtiqueta = vista.findAll('button').find((boton) => boton.text() === 'jazz');
		expect(otraEtiqueta).toBeDefined();
		await otraEtiqueta?.trigger('click');
		await asentar();

		const icono = vista.get('img[alt="Radio Uno"]');
		expect(icono.attributes('src')).toBe('http://si-existe/f.png');
	});
});
