/**
 * Las dos ventanas de resonance, y lo que pasa al cerrarlas.
 *
 * La grande y el mini reproductor tenían cada una su marco escrito a mano.
 * Ahora los dos salen de la librería, con una diferencia que importa: el mini
 * lleva **sólo** el botón de cerrar. Minimizar y maximizar no significan nada
 * en trescientos sesenta píxeles por ciento veinte, y la ventana ni siquiera se
 * puede redimensionar.
 *
 * Y cerrar no es cerrar la ventana y ya: el audio se apaga primero. Sin eso el
 * proceso se va con el dispositivo tomado y el sonido queda cortado a mitad de
 * una nota hasta que el sistema limpia. Es la razón por la que el marco
 * compartido deja reemplazar lo que hace el botón.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { WindowControls, WindowFrame } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import MiniPlayer from '@/components/player/MiniPlayer.vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { invocaciones, laVentanaRecibio, olvidarTodo, romper } from './dobles';

/** La configuración de las ventanas, tal como la lee Tauri al abrirlas. */
const configuracion = (await Bun.file(
	new URL('../src-tauri/tauri.conf.json', import.meta.url)
).json()) as {
	app: { windows: { label: string; width: number; height: number; visible: boolean }[] };
};

let vista: VueWrapper | null = null;

function abrirLaGrande() {
	vista = mount(WindowAppLayout, {
		global: { stubs: { RouterView: true, ResonanceSidebar: true, NowPlayingTopBar: true } },
	});
	return vista;
}

function abrirElMini() {
	vista = mount(MiniPlayer);
	return vista;
}

/** Aprieta el botón de cerrar de la barra. */
async function apretarCerrar(ventana: VueWrapper) {
	await ventana
		.findComponent(WindowControls)
		.find('button[aria-label="windowControls.close"]')
		.trigger('click');
	await ventana.vm.$nextTick();
}

beforeEach(() => setActivePinia(createPinia()));

afterEach(() => {
	vista?.unmount();
	vista = null;
	olvidarTodo();
});

describe('la ventana grande', () => {
	test('usa el marco compartido y no uno propio', () => {
		const ventana = abrirLaGrande();

		expect(ventana.findComponent(WindowFrame).exists()).toBe(true);
		// `rounded-corner-window` es la esquina de la ventana y sale del marco.
		// Con dos, el borde y el fondo se dibujan dos veces y se ven los dos.
		expect(ventana.findAll('.rounded-corner-window').length).toBe(1);
	});

	test('lleva los tres botones', () => {
		expect(abrirLaGrande().findComponent(WindowControls).findAll('button').length).toBe(3);
	});
});

describe('el mini reproductor', () => {
	test('va sin barra: es un widget, no una ventana', () => {
		// Trescientos sesenta píxeles por ciento veinte, siempre encima, sin
		// redimensionar. Una barra con botones se comería un tercio del alto, y
		// los botones no significarían nada: no hay nada que minimizar ni
		// maximizar, y para volver a la ventana grande está el botón del
		// transporte.
		const ventana = abrirElMini();

		expect(ventana.findComponent(WindowFrame).props('hideBar')).toBe(true);
		expect(ventana.findComponent(WindowControls).exists()).toBe(false);
	});

	test('pero el borde y la esquina son los del resto del escritorio', () => {
		// Que es lo que sí hace falta del marco compartido.
		expect(abrirElMini().findAll('.rounded-corner-window').length).toBe(1);
	});

	test('y nada le pide a la ventana que se cierre', async () => {
		// No es que falte el botón: es que desde el widget no se cierra la
		// aplicación. Para eso se vuelve a la ventana grande. Se aprietan
		// **todos** los botones que hay, porque un `close()` colgado de
		// cualquier otro sería el mismo agujero con otra cara.
		const ventana = abrirElMini();

		for (const boton of ventana.findAll('button')) await boton.trigger('click');

		expect(laVentanaRecibio).toEqual([]);
	});
});

describe('cerrar apaga el audio primero', () => {
	test('en la ventana grande', () => {
		const ventana = abrirLaGrande();

		apretarCerrar(ventana);

		// El comando, y **no** un cierre directo: si el botón cerrara la ventana
		// por su cuenta, el proceso se iría con el dispositivo tomado.
		expect(invocaciones).toContain('close_app');
		expect(laVentanaRecibio).not.toContain('close');
	});

	test('si el comando falla, la ventana se cierra igual', async () => {
		// Una ventana que no se puede cerrar es peor que un audio mal apagado.
		romper('close_app');
		const ventana = abrirLaGrande();

		await apretarCerrar(ventana);
		await ventana.vm.$nextTick();
		await Promise.resolve();

		expect(laVentanaRecibio).toContain('close');
	});
});

describe('la ventana del mini reproductor', () => {
	test('sigue siendo del tamaño de un widget', () => {
		// Sin barra no hay que hacerle lugar a nada: el alto se queda donde
		// estaba. Es lo único de esto que no se ve en una prueba montada
		// —`happy-dom` no calcula layout, todo mide cero—, así que queda como
		// una afirmación sobre la configuración.
		const conf = configuracion.app.windows.find((v) => v.label === 'mini-player');

		expect(conf?.height).toBe(120);
		expect(conf?.width).toBe(360);
	});

	test('y aparece escondida, que es como se abre', () => {
		// Se muestra al pasar desde la ventana grande, no al arrancar.
		const conf = configuracion.app.windows.find((v) => v.label === 'mini-player');

		expect(conf?.visible).toBe(false);
	});
});
