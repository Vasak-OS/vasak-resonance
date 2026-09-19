/**
 * Los dobles de lo que sólo existe adentro de la ventana de Tauri.
 *
 * Sin ellos, importar la ventana falla en la primera línea: el marco pide
 * iconos, escucha el cambio de tema y lee la configuración del escritorio.
 */

/** Lo que se le pidió al backend, en orden. */
export const invocaciones: string[] = [];

/** Lo que contesta cada comando; por omisión, nada. */
const respuestas = new Map<string, unknown>([
	// El almacén de Tauri devuelve `[valor, existe]`. Sin esto la lectura
	// revienta al desestructurar, y el error no nombra al comando.
	['plugin:store|get', [undefined, false]],
]);

export function contestar(comando: string, valor: unknown) {
	respuestas.set(comando, valor);
}

/** Hace que un comando falle, para probar el respaldo. */
const rotos = new Set<string>();

export function romper(comando: string) {
	rotos.add(comando);
}

export async function invoke(comando: string) {
	invocaciones.push(comando);
	if (rotos.has(comando)) throw new Error(`el backend rechazó ${comando}`);
	return respuestas.get(comando);
}

export const laVentanaRecibio: string[] = [];

/**
 * El webview actual, que el arrastre de audio pide al montar.
 *
 * `onDragDropEvent` es lo único que se le llama, y devuelve cómo darse de baja.
 */
export function getCurrentWebview() {
	return {
		label: 'main',
		onDragDropEvent: async () => () => {},
	};
}

export function getCurrentWindow() {
	return {
		label: 'main',
		minimize: async () => void laVentanaRecibio.push('minimize'),
		toggleMaximize: async () => void laVentanaRecibio.push('toggleMaximize'),
		close: async () => void laVentanaRecibio.push('close'),
	};
}

/** El `t()` devuelve la clave: una prueba que mire el texto mira la clave. */
export function useI18n() {
	return { t: (clave: string) => clave, locale: { value: 'es' } };
}

export async function readConfig() {
	return {};
}

export function useConfigStore() {
	return { config: {}, loadConfig: async () => {} };
}

export async function listen(_nombre: string, _manejador: () => unknown) {
	return () => {};
}

export async function getIconSource(_nombre: string) {
	return 'icono.png';
}

export async function getSymbolSource(_nombre: string) {
	return 'simbolo.png';
}

export function olvidarTodo() {
	invocaciones.length = 0;
	laVentanaRecibio.length = 0;
	respuestas.clear();
	respuestas.set('plugin:store|get', [undefined, false]);
	rotos.clear();
}
