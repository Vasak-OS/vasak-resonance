import { beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import SettingsView from '@/components/views/SettingsView.vue';
import { getLastfmStatus } from '../src/services/lastfm.service';
import { lastfmStep } from '../src/tools/lastfm';
import { arreglar, contestar, invocaciones, romper } from './dobles';

/**
 * Lo de Last.fm está apagado salvo que alguien lo encienda, y «apagado» tiene
 * que ser además lo que pase cuando algo sale mal: una sección que ofrece
 * vincular una cuenta que no se puede vincular es peor que no tenerla.
 */

describe('en qué punto está la vinculación con Last.fm', () => {
	test('sin clave de API la sección no se dibuja', () => {
		expect(lastfmStep({ configured: false, user: null }, null)).toBe('hidden');
	});

	/** Ni siquiera con una sesión colgada de antes: sin clave no se puede firmar. */
	test('sin clave no se dibuja aunque haya usuario', () => {
		expect(lastfmStep({ configured: false, user: 'alguien' }, 'tok')).toBe('hidden');
	});

	test('con clave y sin autorizar, ofrece vincular', () => {
		expect(lastfmStep({ configured: true, user: null }, null)).toBe('unlinked');
	});

	test('con un token a medio usar, espera la confirmación', () => {
		expect(lastfmStep({ configured: true, user: null }, 'tok')).toBe('authorizing');
	});

	/**
	 * Estar vinculado gana sobre el token pendiente. Si no, confirmar dejaría la
	 * sección pidiendo confirmar para siempre: el token sigue ahí después de
	 * usarlo.
	 */
	test('vinculado gana sobre el token que quedó dando vueltas', () => {
		expect(lastfmStep({ configured: true, user: 'alguien' }, 'tok')).toBe('linked');
	});
});

describe('leer el estado', () => {
	beforeEach(() => {
		invocaciones.length = 0;
		arreglar('lastfm_status');
	});

	test('pasa por el backend, y traduce los nombres del cable', async () => {
		// Rust serializa `configurado` y `usuario` (`lastfm.rs`); la ventana
		// habla de `configured` y `user`, y la traducción vive en el servicio.
		contestar('lastfm_status', { configurado: true, usuario: 'alguien' });

		expect(await getLastfmStatus()).toEqual({ configured: true, user: 'alguien' });
		expect(invocaciones).toContain('lastfm_status');
	});

	test('si el backend falla, queda apagado y no revienta', async () => {
		romper('lastfm_status');

		expect(await getLastfmStatus()).toEqual({ configured: false, user: null });
	});
});

/**
 * Y lo mismo dibujado. Lo de arriba prueba la decisión; esto prueba que la
 * decisión llega a la pantalla, que es donde importa: una sección que se dibuja
 * sin clave de API ofrece vincular algo que no se puede vincular.
 */
describe('la sección de ajustes', () => {
	beforeEach(() => {
		invocaciones.length = 0;
		arreglar('lastfm_status');
		setActivePinia(createPinia());
	});

	/**
	 * `t()` devuelve la clave en las pruebas —no hay plugin de idiomas—, así que
	 * lo que se busca es la clave y no el texto en español.
	 */
	const withStatus = async (status: { configurado: boolean; usuario: string | null }) => {
		contestar('lastfm_status', status);
		const view = mount(SettingsView);
		// El estado se pide al montar y vuelve por una promesa, y los ajustes
		// encadenan otras tres: sin dejar correr la cola de tareas, la sección
		// todavía no se dibujó.
		await new Promise((done) => setTimeout(done, 30));
		await nextTick();
		return view;
	};

	test('sin clave de API no se dibuja', async () => {
		const view = await withStatus({ configurado: false, usuario: null });

		expect(view.text()).not.toContain('settings.lastfm');
	});

	test('con clave y sin vincular, ofrece vincular', async () => {
		const view = await withStatus({ configurado: true, usuario: null });

		expect(view.text()).toContain('settings.lastfmGroup');
		expect(view.text()).toContain('settings.lastfmLink');
		expect(view.text()).not.toContain('settings.lastfmUnlink');
		expect(view.text()).not.toContain('settings.lastfmLinkedAs');
	});

	test('vinculada, dice con quién y ofrece soltarla', async () => {
		const view = await withStatus({ configurado: true, usuario: 'pato' });

		expect(view.text()).toContain('settings.lastfmLinkedAs');
		expect(view.text()).toContain('settings.lastfmUnlink');
	});

	/**
	 * Dos clics seguidos son un pedido, no dos.
	 *
	 * Sin la guarda son dos pestañas del navegador y dos tokens, y el que vuelve
	 * segundo pisa al primero: la persona autoriza uno y se confirma el otro,
	 * que nadie autorizó.
	 */
	test('dos clics en vincular piden una sola autorización', async () => {
		const view = await withStatus({ configurado: true, usuario: null });
		contestar('lastfm_start_authorization', 'token-1');
		invocaciones.length = 0;

		const button = view
			.findAll('button')
			.find((candidate) => candidate.text().includes('settings.lastfmLink'));
		expect(button, 'el botón de vincular tiene que estar').toBeDefined();

		await button?.trigger('click');
		await button?.trigger('click');
		await new Promise((done) => setTimeout(done, 10));

		expect(invocaciones.filter((c) => c === 'lastfm_start_authorization')).toHaveLength(1);
	});

	/**
	 * El nombre entra por `.replace('{0}', …)` y no por `t()`, que en este
	 * taller no interpola. En las pruebas `t()` devuelve la clave, así que el
	 * relleno se comprueba sobre el texto de verdad.
	 */
	test('el nombre entra donde va el hueco', () => {
		expect('Vinculado como {0}'.replace('{0}', 'pato')).toBe('Vinculado como pato');
	});
});
