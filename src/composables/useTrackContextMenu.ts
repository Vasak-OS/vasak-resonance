import { type MenuEntry, useContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { isRevealError, showInFileManager } from '@/services/reveal.service';
import { usePlayerStore } from '@/stores/player';

/**
 * El menú del clic derecho de una lista de canciones.
 *
 * Se engancha uno por lista, no uno por fila: la biblioteca puede tener miles de
 * canciones y el `RecycleScroller` recicla las filas al desplazarse. De qué
 * canción se trata lo dice el DOM —la fila se marca con `data-track-path`—, así
 * que funciona igual con listas virtualizadas.
 *
 * Cada opción hace algo que la aplicación ya sabía hacer: reproducir, encolar,
 * favoritos y abrir la carpeta. No hay nada acá que sólo exista en el menú.
 */
export function useTrackContextMenu() {
	const { t } = useI18n();
	const { show } = useContextMenu();
	const playerStore = usePlayerStore();

	function trackPathAt(event: MouseEvent): string | null {
		const target = event.target;
		const row = target instanceof Element ? target.closest<HTMLElement>('[data-track-path]') : null;

		return row?.dataset.trackPath || null;
	}

	function addToQueue(path: string) {
		// La cola no admite repetidos, así que el aviso dice lo que realmente
		// pasó en vez de dar por hecho que se agregó.
		const before = playerStore.queue.length;
		playerStore.enqueuePaths([path]);
		playerStore.showGlobalBadge(
			playerStore.queue.length > before
				? t('contextMenu.addedToQueue')
				: t('contextMenu.alreadyInQueue')
		);
	}

	async function reveal(path: string) {
		try {
			await showInFileManager(path);
		} catch (error) {
			console.warn('[useTrackContextMenu] no se pudo abrir el gestor de archivos', error);
			// Una canción borrada fuera del reproductor sigue estando en la
			// biblioteca. Decir «no se pudo abrir la carpeta» ahí desorienta: la
			// carpeta está, la que falta es la canción.
			const missingFile = isRevealError(error) && error.kind === 'fileMissing';
			playerStore.showGlobalBadge(
				missingFile ? t('contextMenu.songFileMissing') : t('contextMenu.couldNotShowInFileManager')
			);
		}
	}

	/** Para enganchar con `@contextmenu` en el elemento que contiene la lista. */
	async function onTrackContextMenu(event: MouseEvent) {
		const path = trackPathAt(event);
		if (!path) {
			return;
		}

		// El menú de los campos de texto escucha en el documento: cortarle el
		// evento acá evita que los dos se disputen el mismo clic.
		event.stopPropagation();

		const isFavorite = playerStore.isFavoritePath(path);
		const entries: MenuEntry[] = [
			{ id: 'play', label: t('contextMenu.play'), icon: 'media-playback-start' },
			{ id: 'addToQueue', label: t('contextMenu.addToQueue'), icon: 'list-add' },
			{ type: 'separator' },
			{
				id: 'toggleFavorite',
				label: isFavorite ? t('contextMenu.removeFromFavorites') : t('contextMenu.addToFavorites'),
				icon: isFavorite ? 'non-starred' : 'starred',
			},
			{ type: 'separator' },
			{
				id: 'reveal',
				label: t('contextMenu.showInFileManager'),
				icon: 'system-file-manager',
			},
		];

		const chosen = await show(entries, event);

		switch (chosen?.id) {
			case 'play':
				await playerStore.playDropped(path);
				break;
			case 'addToQueue':
				addToQueue(path);
				break;
			case 'toggleFavorite':
				playerStore.toggleFavoritePath(path);
				break;
			case 'reveal':
				await reveal(path);
				break;
		}
	}

	return { onTrackContextMenu };
}
