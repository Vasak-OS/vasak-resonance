/**
 * La cola de reproducción, como lista de entradas con identidad propia.
 *
 * Guardar sólo la ruta —o peor, la posición— alcanzaba mientras nada se moviera.
 * Pero la cola avanza sola cuando termina una canción, y el menú del clic
 * derecho se queda abierto esperando a que la persona elija. En ese rato la
 * primera entrada se va y todas las demás corren un lugar: quien pidió «quitar»
 * sobre la tercera terminaba quitando la cuarta. Cada entrada lleva entonces un
 * identificador que no cambia mientras la entrada siga en la cola, y el menú
 * habla de la entrada que se señaló, no del lugar que ocupaba.
 *
 * La ruta tampoco serviría de identificador: la misma canción puede entrar dos
 * veces en la cola y «la otra» no es la que se señaló.
 */

export type QueueEntry = {
	/** Único mientras dure la sesión; no se persiste ni se muestra. */
	id: string;
	path: string;
};

let lastQueueEntryId = 0;

/** Una entrada nueva, con un identificador que no le tocó a ninguna otra. */
export const createQueueEntry = (path: string): QueueEntry => {
	lastQueueEntryId += 1;
	return { id: `queue-${lastQueueEntryId}`, path };
};

export const createQueueEntries = (paths: string[]): QueueEntry[] => paths.map(createQueueEntry);

export const queuePaths = (entries: QueueEntry[]): string[] => entries.map((entry) => entry.path);

/** Quita la entrada señalada; las que quedan conservan su identificador. */
export const removeQueueEntry = (entries: QueueEntry[], id: string): QueueEntry[] =>
	entries.filter((entry) => entry.id !== id);

/**
 * Mueve una entrada delante de otra. Se piden identificadores y no posiciones
 * por el mismo motivo que en el menú: entre que se agarra y se suelta, la cola
 * puede haber avanzado.
 */
export const moveQueueEntry = (
	entries: QueueEntry[],
	fromId: string,
	toId: string
): QueueEntry[] => {
	if (fromId === toId) {
		return entries;
	}

	const fromIndex = entries.findIndex((entry) => entry.id === fromId);
	const toIndex = entries.findIndex((entry) => entry.id === toId);
	if (fromIndex < 0 || toIndex < 0) {
		return entries;
	}

	const next = [...entries];
	const [moved] = next.splice(fromIndex, 1);

	// Sacar la entrada corre un lugar hacia la izquierda a todo lo que estaba
	// después, así que para moverla hacia adelante el destino ya no es el mismo
	// número: sin esto, soltar la primera sobre la tercera la dejaba **después**
	// de la tercera en vez de en su lugar.
	const destino = fromIndex < toIndex ? toIndex - 1 : toIndex;

	next.splice(destino, 0, moved);
	return next;
};

export const findQueueEntry = (entries: QueueEntry[], id: string): QueueEntry | undefined =>
	entries.find((entry) => entry.id === id);

/**
 * Cómo se repite lo que suena.
 *
 * Los tres nombres se corresponden con los tres valores que define MPRIS
 * —`None`, `Track`, `Playlist`—, que es lo que lee el panel del escritorio.
 */
export type Repeticion = 'ninguna' | 'uno' | 'todo';

export const REPETICIONES: Repeticion[] = ['ninguna', 'todo', 'uno'];

/** La siguiente en el ciclo del botón. */
export const siguienteRepeticion = (actual: Repeticion): Repeticion =>
	REPETICIONES[(REPETICIONES.indexOf(actual) + 1) % REPETICIONES.length];

/**
 * Qué suena después, y cómo queda la cola.
 *
 * Toda la regla de repetir y del aleatorio vive acá, como función pura, porque
 * es lo único de esto que se puede equivocar en silencio.
 *
 * `azar` se recibe para poder probarla: la reproducción normal le pasa
 * `Math.random`.
 */
export function elegirSiguiente(
	entries: QueueEntry[],
	actual: string | null,
	repeticion: Repeticion,
	aleatorio: boolean,
	azar: () => number = Math.random
): { siguiente: string | null; cola: QueueEntry[] } {
	// Repetir una sola no toca la cola: lo que haya después sigue esperando su
	// turno para cuando se apague.
	if (repeticion === 'uno' && actual) {
		return { siguiente: actual, cola: entries };
	}

	if (entries.length === 0) {
		return { siguiente: null, cola: [] };
	}

	// Con el aleatorio, la elegida sale de cualquier lugar de la cola en vez de
	// la primera. La cola **no se mezcla**: así apagar el aleatorio vuelve al
	// orden de siempre sin tener que acordarse de cuál era.
	const elegida = aleatorio ? Math.floor(clamp01(azar()) * entries.length) : 0;
	const siguiente = entries[elegida];
	const cola = entries.filter((_, indice) => indice !== elegida);

	// Repetir todo: la que se va vuelve al final, y así la cola da vueltas.
	if (repeticion === 'todo' && actual) {
		cola.push(createQueueEntry(actual));
	}

	return { siguiente: siguiente.path, cola };
}

/**
 * Deja el azar dentro de `[0, 1)`.
 *
 * `Math.random` ya cumple, pero esto recibe cualquier función: un 1 devuelto
 * por un doble de prueba —o por una implementación distraída— elegiría un
 * índice que no existe.
 */
const clamp01 = (valor: number): number => {
	if (!Number.isFinite(valor) || valor < 0) {
		return 0;
	}
	return valor >= 1 ? 0.999999 : valor;
};
