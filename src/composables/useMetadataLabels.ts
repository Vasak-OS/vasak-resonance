import { useI18n } from '@vasakgroup/tauri-plugin-i18n';

/**
 * Lo que el backend escribe cuando la canción no trae la etiqueta.
 *
 * No se traducen donde se guardan: `lyrics.rs` los reconoce para no salir a
 * buscar la letra de «Unknown Artist», y el cache de la biblioteca los usa como
 * valor de agrupación. Se traducen sólo al mostrarlos, que es lo que acá se
 * hace, para que la interfaz no quede en dos idiomas.
 */
const UNKNOWN_ARTIST = 'Unknown Artist';
const UNKNOWN_ALBUM = 'Unknown Album';

export function useMetadataLabels() {
	const { t } = useI18n();

	const label = (value: string | null | undefined, sentinel: string, key: string): string => {
		const trimmed = value?.trim() ?? '';
		return !trimmed || trimmed === sentinel ? t(key) : trimmed;
	};

	const artistLabel = (value: string | null | undefined) =>
		label(value, UNKNOWN_ARTIST, 'common.unknownArtist');

	const albumLabel = (value: string | null | undefined) =>
		label(value, UNKNOWN_ALBUM, 'common.unknownAlbum');

	return { artistLabel, albumLabel };
}
