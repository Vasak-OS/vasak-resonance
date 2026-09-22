import { describe, expect, test } from 'bun:test';

/**
 * Los bancos de `herramientas/` repiten a mano la fórmula de las alturas de
 * `PlaybackWaves.vue`, porque corren fuera de la aplicación y no pueden
 * importar el componente.
 *
 * Una copia a mano es una copia que se separa, y ésta se separa **en silencio**:
 * el banco sigue dando números, sólo que de otra cosa. Esto los compara
 * **corriendo la función del banco de verdad** y no buscándole los números
 * adentro: buscar números deja pasar que alguien cambie `Math.sin`, un operador
 * o el orden de los términos.
 */

const componente = await Bun.file('src/components/player/PlaybackWaves.vue').text();

const bancos = [
	'herramientas/bancos/tira-de-barras.html',
	'herramientas/bancos/cadencia-y-listas.html',
];

/** El valor por omisión que declara el componente para esa propiedad. */
function porOmision(nombre: string): number {
	const encontrado = componente.match(new RegExp(`\\b${nombre}:\\s*([0-9.]+)`));
	if (!encontrado) {
		throw new Error(`\`${nombre}\` ya no está entre los valores por omisión del componente`);
	}
	return Number(encontrado[1]);
}

/** La función tal como la escribió el banco, sacada del archivo y ejecutable. */
async function alturaDelBanco(ruta: string): Promise<(i: number, momento: number) => number> {
	const banco = await Bun.file(ruta).text();
	const fuente = banco.match(/function alturaDe[\s\S]*?\n\}/)?.[0];
	if (!fuente) {
		throw new Error(`${ruta} ya no tiene una función \`alturaDe\``);
	}

	return new Function(`${fuente}; return alturaDe;`)() as (i: number, momento: number) => number;
}

/**
 * La misma altura, construida desde los valores del componente.
 *
 * La **forma** de la fórmula está escrita acá a mano, así que un cambio de
 * forma en el componente hay que traerlo también acá: para eso está la prueba
 * de abajo, que exige que el componente siga diciendo lo que esto supone.
 */
function alturaEsperada(i: number, momento: number): number {
	const fase = (i + 1) * porOmision('phaseMultiplier') + momento * porOmision('timeMultiplier');
	const onda = Math.sin(fase) * 0.5 + 0.5;

	return Math.round(porOmision('floorPlaying') + onda * porOmision('amplitude'));
}

describe('el banco mide lo que el reproductor dibuja', () => {
	/** Si esto falla, el componente cambió la **forma** de la fórmula. */
	test('el componente sigue calculando la altura como esto supone', () => {
		expect(componente).toContain('const phase = (i + 1) * phaseMul + pos * timeMul;');
		expect(componente).toContain('const wave = Math.sin(phase) * 0.5 + 0.5;');
		expect(componente).toContain('height: Math.round(floor + wave * amp),');
	});

	test.each(bancos)('%s da exactamente las mismas alturas', async (ruta) => {
		const delBanco = await alturaDelBanco(ruta);

		for (let i = 0; i < 120; i++) {
			for (const momento of [0, 0.5, 1, 7.5, 63, 240.5]) {
				expect(delBanco(i, momento), `barra ${i} en el segundo ${momento}`).toBe(
					alturaEsperada(i, momento)
				);
			}
		}
	});

	/**
	 * Y la carga que se mide tiene que ser la que el reproductor dibuja.
	 *
	 * Cada banco por su lado y buscando **dónde** se usa el número, no si
	 * aparece en algún lado: `110` suelto lo tiene cualquier archivo.
	 */
	test('la tira se mide con la cantidad de barras del reproductor', async () => {
		const banco = await Bun.file('herramientas/bancos/tira-de-barras.html').text();
		const tamanos = banco.match(/const tamanos = \[([^\]]*)\]/)?.[1];

		expect(tamanos, 'el banco ya no declara `tamanos`').toBeDefined();
		expect(
			tamanos?.split(',').map((n) => Number(n.trim())),
			'el tamaño real del reproductor tiene que ser uno de los medidos'
		).toContain(porOmision('steps'));
	});

	test('el banco de cadencia arma la tira del reproductor', async () => {
		const banco = await Bun.file('herramientas/bancos/cadencia-y-listas.html').text();

		expect(banco).toContain(`armarTira(${porOmision('steps')})`);
	});
});
