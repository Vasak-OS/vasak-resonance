import { describe, expect, test } from 'bun:test';
import {
	createQueueEntries,
	findQueueEntry,
	moveQueueEntry,
	queuePaths,
	removeQueueEntry,
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

	test('reordenar con entradas que ya no están deja la cola como estaba', () => {
		const entries = createQueueEntries(['/musica/a.mp3', '/musica/b.mp3']);

		expect(moveQueueEntry(entries, 'no-existe', entries[0].id)).toBe(entries);
		expect(moveQueueEntry(entries, entries[0].id, entries[0].id)).toBe(entries);
	});
});
