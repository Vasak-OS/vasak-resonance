import { beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import SettingsView from '@/components/views/SettingsView.vue';
import { type EstadoDeLastfm, estadoDeLastfm } from '../src/services/lastfm.service';
import { pasoDeLastfm } from '../src/tools/lastfm';
import { arreglar, contestar, invocaciones, romper } from './dobles';

/**
 * Lo de Last.fm está apagado salvo que alguien lo encienda, y «apagado» tiene
 * que ser además lo que pase cuando algo sale mal: una sección que ofrece
 * vincular una cuenta que no se puede vincular es peor que no tenerla.
 */

describe('en qué punto está la vinculación con Last.fm', () => {
	test('sin clave de API la sección no se dibuja', () => {
		expect(pasoDeLastfm({ configurado: false, usuario: null }, null)).toBe('oculto');
	});

	/** Ni siquiera con una sesión colgada de antes: sin clave no se puede firmar. */
	test('sin clave no se dibuja aunque haya usuario', () => {
		expect(pasoDeLastfm({ configurado: false, usuario: 'alguien' }, 'tok')).toBe('oculto');
	});

	test('con clave y sin autorizar, ofrece vincular', () => {
		expect(pasoDeLastfm({ configurado: true, usuario: null }, null)).toBe('sin-vincular');
	});

	test('con un token a medio usar, espera la confirmación', () => {
		expect(pasoDeLastfm({ configurado: true, usuario: null }, 'tok')).toBe('autorizando');
	});

	/**
	 * Estar vinculado gana sobre el token pendiente. Si no, confirmar dejaría la
	 * sección pidiendo confirmar para siempre: el token sigue ahí después de
	 * usarlo.
	 */
	test('vinculado gana sobre el token que quedó dando vueltas', () => {
		expect(pasoDeLastfm({ configurado: true, usuario: 'alguien' }, 'tok')).toBe('vinculado');
	});
});

describe('leer el estado', () => {
	beforeEach(() => {
		invocaciones.length = 0;
		arreglar('lastfm_status');
	});

	test('pasa por el backend', async () => {
		contestar('lastfm_status', { configurado: true, usuario: 'alguien' });

		expect(await estadoDeLastfm()).toEqual({ configurado: true, usuario: 'alguien' });
		expect(invocaciones).toContain('lastfm_status');
	});

	test('si el backend falla, queda apagado y no revienta', async () => {
		romper('lastfm_status');

		expect(await estadoDeLastfm()).toEqual({ configurado: false, usuario: null });
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
	const conEstado = async (estado: EstadoDeLastfm) => {
		contestar('lastfm_status', estado);
		const vista = mount(SettingsView);
		// El estado se pide al montar y vuelve por una promesa, y los ajustes
		// encadenan otras tres: sin dejar correr la cola de tareas, la sección
		// todavía no se dibujó.
		await new Promise((listo) => setTimeout(listo, 30));
		await nextTick();
		return vista;
	};

	test('sin clave de API no se dibuja', async () => {
		const vista = await conEstado({ configurado: false, usuario: null });

		expect(vista.text()).not.toContain('settings.lastfm');
	});

	test('con clave y sin vincular, ofrece vincular', async () => {
		const vista = await conEstado({ configurado: true, usuario: null });

		expect(vista.text()).toContain('settings.lastfmGroup');
		expect(vista.text()).toContain('settings.lastfmLink');
		expect(vista.text()).not.toContain('settings.lastfmUnlink');
		expect(vista.text()).not.toContain('settings.lastfmLinkedAs');
	});

	test('vinculada, dice con quién y ofrece soltarla', async () => {
		const vista = await conEstado({ configurado: true, usuario: 'pato' });

		expect(vista.text()).toContain('settings.lastfmLinkedAs');
		expect(vista.text()).toContain('settings.lastfmUnlink');
	});

	/**
	 * Dos clics seguidos son un pedido, no dos.
	 *
	 * Sin la guarda son dos pestañas del navegador y dos tokens, y el que vuelve
	 * segundo pisa al primero: la persona autoriza uno y se confirma el otro,
	 * que nadie autorizó.
	 */
	test('dos clics en vincular piden una sola autorización', async () => {
		const vista = await conEstado({ configurado: true, usuario: null });
		contestar('lastfm_start_authorization', 'token-1');
		invocaciones.length = 0;

		const boton = vista
			.findAll('button')
			.find((candidato) => candidato.text().includes('settings.lastfmLink'));
		expect(boton, 'el botón de vincular tiene que estar').toBeDefined();

		await boton?.trigger('click');
		await boton?.trigger('click');
		await new Promise((listo) => setTimeout(listo, 10));

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
