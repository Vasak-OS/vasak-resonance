import { beforeEach, describe, expect, test } from 'bun:test';
import { createPinia, setActivePinia } from 'pinia';
import { useSettingsStore } from '../src/stores/settings';
import { contestar, invocaciones } from './dobles';

/**
 * Repetir y el aleatorio se guardan por el mismo motivo que el encadenado:
 * apagar el aleatorio y que vuelva prendido en el próximo arranque es molesto de
 * la misma forma. Y se le avisan al backend, que es de donde MPRIS los lee.
 */

describe('los modos de reproducción', () => {
	/**
	 * Lo que contesta el almacén de Tauri, que es donde se guardan de verdad.
	 *
	 * Devuelve `[valor, existe]`. La primera versión de estas pruebas escribía
	 * en `localStorage` —que es el respaldo para cuando el plugin no está— y
	 * pasaba en verde sin tocar el camino que usa la aplicación.
	 */
	const loQueHayGuardado = (guardado: unknown | null) => {
		contestar('plugin:store|get', guardado === null ? [undefined, false] : [guardado, true]);
	};

	beforeEach(() => {
		localStorage.clear();
		invocaciones.length = 0;
		loQueHayGuardado(null);
		setActivePinia(createPinia());
	});

	test('arrancan apagados', async () => {
		const settings = useSettingsStore();
		await settings.load();

		expect(settings.repeticion).toBe('ninguna');
		expect(settings.aleatorio).toBe(false);
	});

	test('lo que quedó guardado se lee al arrancar', async () => {
		loQueHayGuardado({ repeticion: 'todo', aleatorio: true });

		const settings = useSettingsStore();
		await settings.load();

		expect(settings.repeticion).toBe('todo');
		expect(settings.aleatorio).toBe(true);
	});

	test('elegir un modo lo manda a guardar', async () => {
		const settings = useSettingsStore();
		await settings.load();

		await settings.setRepeticion('uno');

		expect(invocaciones).toContain('plugin:store|set');
		expect(invocaciones).toContain('plugin:store|save');
	});

	test('un modo guardado que no existe no se toma', async () => {
		// Un archivo de ajustes de otra versión, o editado a mano.
		loQueHayGuardado({ repeticion: 'lo-que-sea', aleatorio: true });

		const settings = useSettingsStore();
		await settings.load();

		expect(settings.repeticion).toBe('ninguna');
		expect(settings.aleatorio).toBe(true);
	});

	test('cada cambio se le avisa al backend', async () => {
		// Sin esto MPRIS seguiría contestando lo que contestaba antes, que era
		// un valor fijo: el panel del escritorio dibuja los botones y no pasa
		// nada.
		const settings = useSettingsStore();
		await settings.load();
		const avisosAlCargar = invocaciones.filter((c) => c === 'set_playback_modes').length;

		await settings.setRepeticion('uno');
		await settings.setAleatorio(true);

		expect(invocaciones.filter((c) => c === 'set_playback_modes').length).toBe(avisosAlCargar + 2);
	});

	test('al arrancar también se le avisa, con lo que estaba guardado', async () => {
		// El backend arranca con sus propios valores por omisión: sin este aviso,
		// MPRIS diría «sin repetir» en una sesión que quedó repitiendo.
		loQueHayGuardado({ repeticion: 'todo', aleatorio: true });

		const settings = useSettingsStore();
		await settings.load();

		expect(invocaciones).toContain('set_playback_modes');
		expect(settings.repeticion).toBe('todo');
	});
});
