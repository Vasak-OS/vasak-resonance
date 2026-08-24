import { invoke } from '@tauri-apps/api/core';

/**
 * Por qué no se pudo mostrar la canción, tal como lo cuenta el backend.
 *
 * Se mira el código y no el texto para que el aviso salga traducido.
 */
export type RevealErrorKind =
	| 'noContainingFolder'
	| 'folderMissing'
	| 'fileMissing'
	| 'launchFailed';

export type RevealError = {
	kind: RevealErrorKind;
	detail: string;
};

const REVEAL_ERROR_KINDS: RevealErrorKind[] = [
	'noContainingFolder',
	'folderMissing',
	'fileMissing',
	'launchFailed',
];

/** Distingue el error del backend de cualquier otra cosa que pueda fallar. */
export function isRevealError(value: unknown): value is RevealError {
	if (typeof value !== 'object' || value === null) {
		return false;
	}

	const kind = (value as { kind?: unknown }).kind;
	return typeof kind === 'string' && REVEAL_ERROR_KINDS.includes(kind as RevealErrorKind);
}

/**
 * Abre en el gestor de archivos la carpeta que contiene la canción.
 *
 * Quien elige la carpeta y la abre es el backend: acá sólo se le pasa la ruta
 * del archivo tal como la tiene la biblioteca.
 */
export async function showInFileManager(path: string): Promise<void> {
	await invoke('show_in_file_manager', { path });
}
