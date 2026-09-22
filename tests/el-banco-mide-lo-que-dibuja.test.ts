import { describe, expect, test } from 'bun:test';

/**
 * Los bancos de `herramientas/` repiten a mano la fórmula de las alturas de
 * `PlaybackWaves.vue`, porque corren fuera de la aplicación y no pueden
 * importar el componente.
 *
 * Una copia a mano es una copia que se separa, y ésta se separa **en silencio**:
 * el banco sigue dando números, sólo que de otra cosa. Esto los compara.
 */

const componente = await Bun.file('src/components/player/PlaybackWaves.vue').text();
const bancos = [
	'herramientas/bancos/tira-de-barras.html',
	'herramientas/bancos/cadencia-y-listas.html',
];

/** El valor por omisión que declara el componente para esa propiedad. */
function porOmision(nombre: string): string {
	const encontrado = componente.match(new RegExp(`\\b${nombre}:\\s*([0-9.]+)`));
	if (!encontrado) {
		throw new Error(`\`${nombre}\` ya no está entre los valores por omisión del componente`);
	}
	return encontrado[1];
}

describe('el banco mide lo que el reproductor dibuja', () => {
	test('el componente sigue declarando las constantes de la fórmula', () => {
		expect(porOmision('amplitude')).toBe('11');
		expect(porOmision('phaseMultiplier')).toBe('0.65');
		expect(porOmision('timeMultiplier')).toBe('0.38');
		expect(porOmision('floorPlaying')).toBe('5');
		expect(porOmision('steps')).toBe('110');
	});

	/**
	 * La fórmula del banco es
	 * `round(floorPlaying + (sin((i + 1) * phaseMultiplier + t * timeMultiplier) * 0.5 + 0.5) * amplitude)`,
	 * así que los cuatro números tienen que estar en su `alturaDe`.
	 */
	test.each(bancos)('%s usa los mismos números', async (ruta) => {
		const banco = await Bun.file(ruta).text();
		const formula = banco.match(/function alturaDe[\s\S]*?\n\}/)?.[0];

		expect(formula, `${ruta} ya no tiene \`alturaDe\``).toBeDefined();

		for (const nombre of ['amplitude', 'phaseMultiplier', 'timeMultiplier', 'floorPlaying']) {
			expect(formula, `${ruta} no usa ${nombre}=${porOmision(nombre)}`).toContain(
				porOmision(nombre)
			);
		}
	});

	/** Y la carga que se mide tiene que ser la que el reproductor dibuja. */
	test('la tira se mide con la cantidad de barras que usa el reproductor', async () => {
		const banco = await Bun.file('herramientas/bancos/tira-de-barras.html').text();

		expect(banco).toContain(`['110 barras, height', ${porOmision('steps')}, conAltura]`);
	});
});
