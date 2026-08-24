/**
 * Lo que Discord tiene que mostrar.
 *
 * Discord no necesita que le cuenten el paso del tiempo: se le dan las marcas de
 * inicio y de fin y él dibuja la barra solo. Por eso avisar en cada tic de
 * reproducción —una vez por segundo— sería mandar un mensaje por el socket para
 * decirle algo que ya sabe.
 */
export interface EstadoParaDiscord {
	path: string;
	title: string;
	artist: string;
	albumArtUrl: string | null;
	isPaused: boolean;
	positionSeconds: number;
	durationSeconds: number;
}

/**
 * Cuánto puede desviarse la posición real de la que Discord está mostrando
 * antes de que valga la pena corregirla.
 *
 * Tres segundos: un salto en la barra siempre lo supera, y el ruido normal
 * —tics que llegan un poco tarde, redondeos— nunca.
 */
const TOLERANCIA_SEGUNDOS = 3;

/**
 * Si hace falta avisarle a Discord.
 *
 * Cambió la canción, se pausó o se reanudó, o alguien saltó a otro punto: eso es
 * todo lo que Discord no puede deducir solo. `segundosDesdeElAviso` es cuánto
 * pasó desde el último mensaje, y sirve para saber dónde *creería* Discord que
 * va la canción; si la posición real se le fue lejos, es que hubo un salto.
 */
export function hayQueAvisar(
	anterior: EstadoParaDiscord | null,
	actual: EstadoParaDiscord,
	segundosDesdeElAviso: number
): boolean {
	if (!anterior) return true;
	if (anterior.path !== actual.path) return true;
	if (anterior.isPaused !== actual.isPaused) return true;

	// En pausa la barra no corre, así que lo que Discord muestra es lo mismo
	// que se le mandó.
	const avance = anterior.isPaused ? 0 : segundosDesdeElAviso;
	const dondeCreeQueVa = anterior.positionSeconds + avance;

	return Math.abs(actual.positionSeconds - dondeCreeQueVa) > TOLERANCIA_SEGUNDOS;
}
