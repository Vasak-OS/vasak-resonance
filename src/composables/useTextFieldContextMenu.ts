import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { type MenuEntry, useContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { onMounted, onUnmounted } from 'vue';

type TextField = HTMLInputElement | HTMLTextAreaElement;

/**
 * Los tipos de `input` en los que el navegador da la selección de verdad
 * (`selectionStart`/`selectionEnd`). En `number` o `email` esas propiedades
 * lanzan una excepción, y en `range` o `checkbox` no hay texto que copiar.
 */
const SELECTABLE_TYPES = new Set(['text', 'search', 'url', 'tel']);

function asTextField(target: EventTarget | null): TextField | null {
	if (!(target instanceof Element)) {
		return null;
	}

	const candidate = target.closest('input, textarea');
	if (candidate instanceof HTMLTextAreaElement) {
		return candidate;
	}

	if (candidate instanceof HTMLInputElement && SELECTABLE_TYPES.has(candidate.type)) {
		return candidate;
	}

	return null;
}

/**
 * El menú del clic derecho de los campos de texto de toda la aplicación.
 *
 * Se engancha una sola vez —en `App.vue`— y con un escucha en el documento, no
 * alrededor de cada campo. Hay seis campos repartidos en cinco pantallas y el
 * que se agregue mañana tendría que acordarse de esto; y envolver un `input`
 * cambia su bloque contenedor, que en las grillas y los `flex` de Resonance lo
 * deforma.
 *
 * En burbujeo a propósito: así corre último. Un menú más específico —el de una
 * canción— corta la propagación en su propio elemento, y entonces el evento no
 * llega hasta acá y el suyo es el que se abre.
 */
export function useTextFieldContextMenu() {
	const { t } = useI18n();
	const { show } = useContextMenu();

	function selectedText(field: TextField): string {
		return field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0);
	}

	/**
	 * Reemplaza lo seleccionado (o inserta donde está el cursor) y avisa con un
	 * evento `input`: es el que escucha `v-model`, y sin él el campo mostraría
	 * una cosa mientras el filtro sigue usando otra.
	 */
	function replaceSelection(field: TextField, text: string) {
		const start = field.selectionStart ?? field.value.length;
		const end = field.selectionEnd ?? start;

		field.value = `${field.value.slice(0, start)}${text}${field.value.slice(end)}`;

		const caret = start + text.length;
		field.setSelectionRange(caret, caret);
		field.dispatchEvent(new Event('input', { bubbles: true }));
		field.focus();
	}

	async function handleContextMenu(event: MouseEvent) {
		const field = asTextField(event.target);
		if (!field) {
			return;
		}

		// Sin selección, copiar y cortar no harían nada: no aparecen.
		const selection = selectedText(field);
		const entries: MenuEntry[] = [];

		if (selection) {
			entries.push(
				{
					id: 'copy',
					label: t('contextMenu.copy'),
					icon: 'edit-copy',
					accelerator: 'Ctrl+C',
				},
				{ id: 'cut', label: t('contextMenu.cut'), icon: 'edit-cut', accelerator: 'Ctrl+X' }
			);
		}

		entries.push(
			{ id: 'paste', label: t('contextMenu.paste'), icon: 'edit-paste', accelerator: 'Ctrl+V' },
			{ type: 'separator' },
			{
				id: 'selectAll',
				label: t('contextMenu.selectAll'),
				icon: 'edit-select-all',
				accelerator: 'Ctrl+A',
			}
		);

		const chosen = await show(entries, event);

		switch (chosen?.id) {
			case 'copy':
				await writeText(selection);
				break;
			case 'cut':
				await writeText(selection);
				replaceSelection(field, '');
				break;
			case 'paste': {
				// El portapapeles puede estar vacío o tener algo que no es
				// texto, y en ese caso leerlo falla: no es un error que valga
				// contarle a nadie, el campo simplemente queda como estaba.
				const text = await readText().catch(() => '');
				if (text) {
					replaceSelection(field, text);
				}
				break;
			}
			case 'selectAll':
				field.focus();
				field.select();
				break;
		}
	}

	function listener(event: MouseEvent) {
		void handleContextMenu(event);
	}

	onMounted(() => document.addEventListener('contextmenu', listener));
	onUnmounted(() => document.removeEventListener('contextmenu', listener));
}
