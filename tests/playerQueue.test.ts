import { describe, expect, test } from 'bun:test';
import {
	createQueueEntries,
	elegirSiguiente,
	findQueueEntry,
	moveQueueEntry,
	queuePaths,
	removeQueueEntry,
	siguienteRepeticion,
} from '../src/stores/playerQueue';

/** La cola avanza sola: la canción que sonaba terminó y sale la primera. */
const advance = <T>(entries: T[]): T[] => entries.slice(1);

describe('la cola de reproducción', () => {
	test('cada entrada tiene un identificador propio aunque se repita la canción', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/a.mp3']);

		expect(entries[0].id).not.toBe(entries[1].id);
		expect(queuePaths(entries)).toEqual(['/musica/a.mp3', '/musica/a.mp3']);
	});

	test('quitar desde el menú saca la canción señalada aunque la cola haya avanzado', () => {
		let entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3', '/musica/c.mp3']);

		// Clic derecho sobre «c»: el menú queda abierto mientras la persona lee.
		const señalada = entries[2];
		expect(señalada.path).toBe('/musica/c.mp3');

		// Mientras tanto termina la canción en curso y la cola corre un lugar.
		entries = advance(entries);

		// Recién ahora se elige «quitar de la cola».
		entries = removeQueueEntry(entries, señalada.id);

		expect(queuePaths(entries)).toEqual(['/musica/b.mp3']);
	});

	test('con la posición en vez del identificador se habría quitado la otra canción', () => {
		// El error que se está evitando, escrito para que se vea la diferencia.
		const inicial = createQueueEntries([
			'/musica/a.mp3',
			'/musica/b.mp3',
			'/musica/c.mp3',
			'/musica/d.mp3',
		]);
		const posiciónSeñalada = 2;
		const trasAvanzar = advance(inicial);

		const porPosición = trasAvanzar.filter((_, index) => index !== posiciónSeñalada);

		expect(queuePaths(porPosición)).toEqual(['/musica/b.mp3', '/musica/c.mp3']);
		expect(queuePaths(porPosición)).not.toContain('/musica/d.mp3');

		const porIdentificador = removeQueueEntry(trasAvanzar, inicial[posiciónSeñalada].id);
		expect(queuePaths(porIdentificador)).toEqual(['/musica/b.mp3', '/musica/d.mp3']);
	});

	test('reproducir ahora encuentra la entrada señalada tras avanzar la cola', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3', '/musica/c.mp3']);
		const señalada = entries[2];

		const trasAvanzar = advance(entries);

		expect(findQueueEntry(trasAvanzar, señalada.id)?.path).toBe('/musica/c.mp3');
	});

	test('una entrada que ya no está en la cola no se encuentra ni se quita', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3']);
		const señalada = entries[0];

		// La canción señalada es justo la que empieza a sonar.
		const trasAvanzar = advance(entries);

		expect(findQueueEntry(trasAvanzar, señalada.id)).toBeUndefined();
		expect(queuePaths(removeQueueEntry(trasAvanzar, señalada.id))).toEqual(['/musica/b.mp3']);
	});

	test('reordenar arrastra la entrada agarrada aunque la cola haya avanzado', () => {
		const entries = createQueueEntries([
			'/musica/a.mp3',
			'/musica/b.mp3',
			'/musica/c.mp3',
			'/musica/d.mp3',
		]);
		const agarrada = entries[3];
		const destino = entries[1];

		const trasAvanzar = advance(entries);
		const reordenada = moveQueueEntry(trasAvanzar, agarrada.id, destino.id);

		expect(queuePaths(reordenada)).toEqual(['/musica/d.mp3', '/musica/b.mp3', '/musica/c.mp3']);
	});

	test('mover hacia adelante deja la entrada en el lugar de la de destino', () => {
		// Sacarla corre un lugar a todo lo que estaba después: sin compensarlo,
		// soltar la primera sobre la tercera la dejaba después de la tercera.
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3', '/musica/c.mp3']);

		const reordenada = moveQueueEntry(entries, entries[0].id, entries[2].id);

		expect(queuePaths(reordenada)).toEqual(['/musica/b.mp3', '/musica/a.mp3', '/musica/c.mp3']);
	});

	test('mover hacia atrás también deja la entrada en el lugar de la de destino', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3', '/musica/c.mp3']);

		const reordenada = moveQueueEntry(entries, entries[2].id, entries[0].id);

		expect(queuePaths(reordenada)).toEqual(['/musica/c.mp3', '/musica/a.mp3', '/musica/b.mp3']);
	});

	test('reordenar con entradas que ya no están deja la cola como estaba', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3']);

		expect(moveQueueEntry(entries, 'no-existe', entries[0].id)).toBe(entries);
		expect(moveQueueEntry(entries, entries[0].id, entries[0].id)).toBe(entries);
	});
});

describe('qué suena después', () => {
	const cola = (...paths: string[]) => createQueueEntries(paths);

	test('sin repetir ni aleatorio, la primera de la cola', () => {
		const { siguiente, cola: quedan } = elegirSiguiente(
			cola('/m/a.mp3', '/m/b.mp3'),
			'/m/actual.mp3',
			'ninguna',
			false
		);

		expect(siguiente).toBe('/m/a.mp3');
		expect(queuePaths(quedan)).toEqual(['/m/b.mp3']);
	});

	test('repetir una vuelve a la misma y no toca la cola', () => {
		// Lo que haya después sigue esperando su turno para cuando se apague.
		const original = cola('/m/a.mp3', '/m/b.mp3');

		const { siguiente, cola: quedan } = elegirSiguiente(original, '/m/actual.mp3', 'uno', false);

		expect(siguiente).toBe('/m/actual.mp3');
		expect(queuePaths(quedan)).toEqual(['/m/a.mp3', '/m/b.mp3']);
	});

	test('repetir todo manda al final la que se va', () => {
		const { siguiente, cola: quedan } = elegirSiguiente(
			cola('/m/a.mp3', '/m/b.mp3'),
			'/m/actual.mp3',
			'todo',
			false
		);

		expect(siguiente).toBe('/m/a.mp3');
		expect(queuePaths(quedan)).toEqual(['/m/b.mp3', '/m/actual.mp3']);
	});

	test('con repetir todo la cola da vueltas y no se agota', () => {
		let actual = '/m/a.mp3';
		let quedan = cola('/m/b.mp3', '/m/c.mp3');
		const sonaron: string[] = [];

		for (let vuelta = 0; vuelta < 6; vuelta += 1) {
			const paso = elegirSiguiente(quedan, actual, 'todo', false);
			actual = paso.siguiente as string;
			quedan = paso.cola;
			sonaron.push(actual);
		}

		expect(sonaron).toEqual([
			'/m/b.mp3',
			'/m/c.mp3',
			'/m/a.mp3',
			'/m/b.mp3',
			'/m/c.mp3',
			'/m/a.mp3',
		]);
	});

	test('el aleatorio elige de cualquier lugar, sin mezclar la cola', () => {
		// Apagar el aleatorio tiene que devolver el orden de siempre, así que la
		// cola no se toca: sólo cambia de dónde se saca la que sigue.
		const { siguiente, cola: quedan } = elegirSiguiente(
			cola('/m/a.mp3', '/m/b.mp3', '/m/c.mp3'),
			null,
			'ninguna',
			true,
			() => 0.7
		);

		expect(siguiente).toBe('/m/c.mp3');
		expect(queuePaths(quedan)).toEqual(['/m/a.mp3', '/m/b.mp3']);
	});

	test('el aleatorio llega a todas las posiciones', () => {
		const elegidas = new Set<string>();
		for (const azar of [0, 0.4, 0.9]) {
			const { siguiente } = elegirSiguiente(
				cola('/m/a.mp3', '/m/b.mp3', '/m/c.mp3'),
				null,
				'ninguna',
				true,
				() => azar
			);
			elegidas.add(siguiente as string);
		}

		expect(elegidas.size).toBe(3);
	});

	test('un azar que devuelve 1 no se sale de la cola', () => {
		// `Math.random` nunca devuelve 1, pero esto recibe cualquier función.
		const { siguiente } = elegirSiguiente(cola('/m/a.mp3'), null, 'ninguna', true, () => 1);

		expect(siguiente).toBe('/m/a.mp3');
	});

	test('con la cola vacía no suena nada, salvo que se repita una', () => {
		expect(elegirSiguiente([], '/m/actual.mp3', 'ninguna', false).siguiente).toBeNull();
		expect(elegirSiguiente([], '/m/actual.mp3', 'todo', false).siguiente).toBeNull();
		expect(elegirSiguiente([], '/m/actual.mp3', 'uno', false).siguiente).toBe('/m/actual.mp3');
	});

	test('el botón de repetir cicla por los tres estados', () => {
		expect(siguienteRepeticion('ninguna')).toBe('todo');
		expect(siguienteRepeticion('todo')).toBe('uno');
		expect(siguienteRepeticion('uno')).toBe('ninguna');
	});
});
