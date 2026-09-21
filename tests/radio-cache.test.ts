import { beforeEach, describe, expect, test } from 'bun:test';
import {
	getCachedStations,
	getStaleCachedStations,
	type RadioStation,
	setCachedStations,
} from '../src/services/radio.service';

/**
 * El caché de emisoras vive en la ventana. Tenía dos problemas que no se ven
 * mirándolo: era uno solo para todas las etiquetas, y lo viejo caducaba también
 * para el caso de «no hay red», que es justo cuando hace falta.
 */

const emisora = (name: string): RadioStation => ({
	uuid: `uuid-${name}`,
	name,
	url: `http://emisora.ejemplo/${name}`,
});

/** Mete algo en el caché con la antigüedad que se le pida. */
const guardarCon = (tags: string[], stations: RadioStation[], hace_ms: number) => {
	setCachedStations(tags, stations);
	const clave = `radio_stations_cache:${tags.join(',')}`;
	const guardado = JSON.parse(localStorage.getItem(clave) as string);
	guardado.timestamp = Date.now() - hace_ms;
	localStorage.setItem(clave, JSON.stringify(guardado));
};

const UNA_HORA = 3_600_000;

describe('el caché de emisoras', () => {
	beforeEach(() => {
		localStorage.clear();
	});

	test('cada etiqueta guarda lo suyo', () => {
		// Con una sola entrada para todas, cambiar el filtro mostraba las
		// emisoras de la etiqueta anterior.
		setCachedStations(['jazz'], [emisora('una de jazz')]);
		setCachedStations(['rock'], [emisora('una de rock')]);

		expect(getCachedStations(['jazz'])?.[0].name).toBe('una de jazz');
		expect(getCachedStations(['rock'])?.[0].name).toBe('una de rock');
	});

	test('una etiqueta sin nada guardado no devuelve la de otra', () => {
		setCachedStations(['jazz'], [emisora('una de jazz')]);

		expect(getCachedStations(['cumbia'])).toBeNull();
	});

	test('la clave no depende del orden ni de las mayúsculas', () => {
		setCachedStations(['Jazz', 'Blues'], [emisora('una')]);

		expect(getCachedStations(['blues', 'jazz'])).not.toBeNull();
	});

	test('lo guardado hace un rato ya no cuenta como fresco', () => {
		guardarCon(['jazz'], [emisora('vieja')], UNA_HORA + 1000);

		expect(getCachedStations(['jazz'])).toBeNull();
	});

	test('pero sigue estando para cuando no hay red', () => {
		// Antes esto caducaba igual, así que sin directorio y una hora después
		// no quedaba nada que mostrar.
		guardarCon(['jazz'], [emisora('vieja')], UNA_HORA * 50);

		expect(getStaleCachedStations(['jazz'])?.[0].name).toBe('vieja');
	});

	test('un caché vacío no se toma por bueno', () => {
		// Si no, la vista se queda mostrando una lista vacía sin preguntar.
		setCachedStations(['jazz'], []);

		expect(getCachedStations(['jazz'])).toBeNull();
		expect(getStaleCachedStations(['jazz'])).toBeNull();
	});

	test('un caché corrupto no rompe la vista', () => {
		localStorage.setItem('radio_stations_cache:jazz', 'esto no es JSON');

		expect(getCachedStations(['jazz'])).toBeNull();
		expect(getStaleCachedStations(['jazz'])).toBeNull();
	});
});
