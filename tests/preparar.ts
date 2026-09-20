/**
 * Lo que tiene que estar listo antes de la primera prueba.
 *
 * El DOM, el complemento que compila los `.vue` —Bun los trata como un archivo
 * suelto y lo que se importa sin él es la ruta— y los dobles de Tauri, que los
 * componentes llaman al importarse.
 */

import { mock } from 'bun:test';
import { GlobalRegistrator } from '@happy-dom/global-registrator';
import './complemento-vue';
import {
	getCurrentWebview,
	getCurrentWindow,
	getIconSource,
	getSymbolSource,
	invoke,
	listen,
	readConfig,
	useConfigStore,
	useI18n,
} from './dobles';

GlobalRegistrator.register();

// El doble **encima** del módulo de verdad, no en su lugar. Reemplazarlo entero
// deja sin exportar lo que no se nombra acá —`SERIALIZE_TO_IPC_FN`, por
// ejemplo— y ahí lo que falla es el import y no la prueba.
const core = await import('@tauri-apps/api/core');
const eventos = await import('@tauri-apps/api/event');
const ventana = await import('@tauri-apps/api/window');
const webview = await import('@tauri-apps/api/webview');
const configuracion = await import('@vasakgroup/plugin-config-manager');
const iconos = await import('@vasakgroup/plugin-vicons');

mock.module('@tauri-apps/api/core', () => ({ ...core, invoke }));
mock.module('@tauri-apps/api/event', () => ({ ...eventos, listen }));
// El doble encima del módulo de verdad: alguien importa `Window` de acá —el
// servicio de ventanas lo hace—, y reemplazarlo entero deja esa exportación sin
// existir, con un error de sintaxis que no nombra a nadie.
mock.module('@tauri-apps/api/window', () => ({ ...ventana, getCurrentWindow }));
mock.module('@tauri-apps/api/webview', () => ({ ...webview, getCurrentWebview }));
mock.module('@vasakgroup/tauri-plugin-i18n', () => ({ useI18n }));
mock.module('@vasakgroup/plugin-vicons', () => ({ ...iconos, getIconSource, getSymbolSource }));
mock.module('@vasakgroup/plugin-config-manager', () => ({
	...configuracion,
	useConfigStore,
	readConfig,
}));
