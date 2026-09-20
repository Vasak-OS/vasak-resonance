import { beforeEach, describe, expect, test } from 'bun:test';
import { createPinia, setActivePinia } from 'pinia';
import { usePlayerStore } from '../src/stores/player';
import { contestar, invocaciones } from './dobles';

/**
 * La tapa que se le manda a Discord se busca por álbum, una vez, y sólo si la
 * presencia está encendida. Las dos cosas son deliberadas: la búsqueda sale a la
 * red y le manda el nombre del álbum a un servicio ajeno.
 *
 * Las respuestas del backend se dejan pendientes a propósito. Una que resuelve
 * sola no prueba nada de lo de acá: para cuando vuelve a sonar el primer álbum
 * ya está en la tabla, y la búsqueda repetida —que es lo que se quiere impedir—
 * no llega a existir.
 */

const TAPA = 'https://coverartarchive.org/release/abc/front';

/** Un `fetch_album_cover_command` que no contesta hasta que se lo diga. */
function respuestaPendiente() {
	let resolver: (valor: unknown) => void = () => {};
	const pendiente = new Promise((resolve) => {
		resolver = resolve;
	});
	contestar('fetch_album_cover_command', pendiente);
	return (remoteUrl: string) =>
		resolver({ cover_data_url: '', dominant_color: '', remote_url: remoteUrl });
}

/** Lo que llega cuando empieza a sonar algo. */
const sonando = (path: string, album: string) => ({
	path,
	position_seconds: 0,
	duration_seconds: 200,
	is_playing: true,
	is_paused: false,
	volume: 1,
	now_playing: {
		path,
		title: 'Un tema',
		artist: 'Un artista',
		album,
		duration_seconds: 200,
		cover_data_url: null,
		dominant_color: null,
	},
});

const busquedas = () => invocaciones.filter((c) => c === 'fetch_album_cover_command').length;

describe('la tapa que se le manda a Discord', () => {
	beforeEach(() => {
		setActivePinia(createPinia());
		invocaciones.length = 0;
	});

	test('con la presencia apagada no se sale a la red', async () => {
		// Es el caso por omisión: sin identificador de aplicación configurado,
		// el hilo de Discord ni siquiera arranca. Quien no usa la presencia no
		// tiene por qué mandarle a nadie los álbumes que escucha.
		contestar('discord_presence_activa', false);
		respuestaPendiente();
		const store = usePlayerStore();

		contestar('get_playback_snapshot', sonando('/m/a.mp3', 'Álbum apagado'));
		await store.syncPlaybackSnapshot();
		await Promise.resolve();

		expect(busquedas()).toBe(0);
	});

	test('el mismo álbum no se busca dos veces aunque vuelva a sonar', async () => {
		// Una cola de dos que alterna, o alguien yendo y viniendo: entre que
		// arranca la búsqueda del primero y termina, suena el segundo y vuelve
		// el primero.
		contestar('discord_presence_activa', true);
		respuestaPendiente();
		const store = usePlayerStore();

		contestar('get_playback_snapshot', sonando('/m/a.mp3', 'Álbum A'));
		await store.syncPlaybackSnapshot();
		await Promise.resolve();

		contestar('get_playback_snapshot', sonando('/m/b.mp3', 'Álbum B'));
		await store.syncPlaybackSnapshot();
		await Promise.resolve();

		contestar('get_playback_snapshot', sonando('/m/a.mp3', 'Álbum A'));
		await store.syncPlaybackSnapshot();
		await Promise.resolve();

		// Dos álbumes, dos búsquedas: la segunda vuelta del primero no agrega
		// una tercera.
		expect(busquedas()).toBe(2);
	});

	test('cuando aparece la tapa se le avisa a Discord', async () => {
		contestar('discord_presence_activa', true);
		const contestarCon = respuestaPendiente();
		const store = usePlayerStore();

		contestar('get_playback_snapshot', sonando('/m/c.mp3', 'Álbum que sí tiene tapa'));
		await store.syncPlaybackSnapshot();
		await Promise.resolve();

		const avisosAntes = invocaciones.filter((c) => c === 'update_discord_presence').length;

		contestarCon(TAPA);
		// Dos vueltas: una para la respuesta del backend y otra para el aviso
		// que se dispara cuando la tapa aterriza.
		await Promise.resolve();
		await Promise.resolve();
		await Promise.resolve();

		expect(invocaciones.filter((c) => c === 'update_discord_presence').length).toBeGreaterThan(
			avisosAntes
		);
	});
});
