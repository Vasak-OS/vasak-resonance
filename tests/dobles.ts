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

/**
 * Lo vuelve a arreglar.
 *
 * `romper` no tenía vuelta atrás y el conjunto es de todo el archivo, así que
 * una prueba que rompía un comando se lo dejaba roto a las que venían después
 * —que fallaban por algo que no estaban probando—.
 */
export function arreglar(comando: string) {
	rotos.delete(comando);
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

/** Los oyentes registrados por evento, para poder dispararlos desde una prueba. */
const listeners = new Map<string, Set<(evento: { payload: unknown }) => unknown>>();

export async function listen(nombre: string, manejador: (evento: { payload: unknown }) => unknown) {
	const suyos = listeners.get(nombre) ?? new Set();
	suyos.add(manejador);
	listeners.set(nombre, suyos);
	return () => {
		suyos.delete(manejador);
	};
}

/** Dispara un evento del backend y espera a que lo atiendan. */
export async function emit(nombre: string, payload: unknown = null) {
	for (const manejador of [...(listeners.get(nombre) ?? [])]) {
		await manejador({ payload });
	}
}

/**
 * Los iconos del tema, devolviendo **qué nombre** se pidió.
 *
 * Una ruta fija —`'icono.png'`— hace que el icono se dibuje, que es lo único
 * que hacía falta mientras nadie miraba cuál era. Ahora que los iconos se piden
 * por nombre, una prueba que quiera comprobar que el botón de silencio pide el
 * altavoz mudo y no el de volumen alto necesita ver el nombre en el `src`.
 */
/**
 * Lo que el tema contesta para un nombre, cuando la prueba lo dice.
 *
 * Sin esto el doble contesta siempre lo mismo y un cambio de tema no se puede
 * comprobar: la fuente sale igual antes y después, así que la prueba pasaría
 * con el componente desconectado del tema.
 */
const themeIcons = new Map<string, string>();

/** Pone —o cambia— lo que el tema devuelve para un nombre. */
export function setThemeIcon(nombre: string, fuente: string) {
	themeIcons.set(nombre, fuente);
}

export async function getIconSource(nombre: string) {
	return themeIcons.get(nombre) ?? `icono:${nombre}`;
}

export async function getSymbolSource(nombre: string) {
	return themeIcons.get(nombre) ?? `simbolo:${nombre}`;
}

export function olvidarTodo() {
	invocaciones.length = 0;
	laVentanaRecibio.length = 0;
	respuestas.clear();
	respuestas.set('plugin:store|get', [undefined, false]);
	rotos.clear();
	listeners.clear();
	themeIcons.clear();
}
