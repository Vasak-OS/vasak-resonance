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
	next.splice(toIndex, 0, moved);
	return next;
};

export const findQueueEntry = (entries: QueueEntry[], id: string): QueueEntry | undefined =>
	entries.find((entry) => entry.id === id);
