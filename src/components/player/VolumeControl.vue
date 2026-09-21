<script setup lang="ts">
/**
 * El volumen de la barra de arriba.
 *
 * ── Lo que le faltaba ───────────────────────────────────────────────────────
 *
 * El `<input type="range">` no tenía nombre: ni `aria-label` ni un `<label>`
 * asociado. Se anunciaba «control deslizante, 47» sin decir de qué, justo al
 * lado del botón que tampoco lo decía. Y el número no tenía unidad —
 * `aria-valuetext` es lo que hace que se oiga «47 %» en vez de «47».
 *
 * El altavoz era un `<svg>` dibujado a mano, o sea un icono que no sigue al
 * tema del escritorio y que además mentía: era el mismo trazo con el volumen
 * en cero que al máximo. Ahora sale del tema y dice en cuál de los tres tramos
 * está.
 *
 * Y el altavoz con su porcentaje era un `<button>` **sin `@click`**: un control
 * que se podía enfocar y apretar, que un lector de pantalla anunciaba como
 * botón, y que no hacía nada. Es un `<span>`, que es lo que siempre fue.
 *
 * ── Por qué no es `SliderControl` ───────────────────────────────────────────
 *
 * La librería tiene el deslizador con esta misma cuenta —`SliderControl`— y es
 * de donde sale el contrato de accesibilidad de acá. No se adopta por dos
 * cosas, anotadas en vue-libvasak#52: pide el icono como **ruta ya resuelta**
 * en vez de por nombre, que es lo contrario de lo que hace el resto de la
 * librería, y viene en un solo tamaño —`w-full p-4`— pensado para una fila de
 * preferencias. Acá el control vive en una barra de título y mide `w-36`.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();

/** El almacén guarda 0..2; el deslizador muestra 0..100. */
const volumePercent = computed(() => Math.round((playerStore.volume || 0) * 50));

/** El altavoz dice en qué tramo está, que es lo que un trazo fijo no decía. */
const volumeIcon = computed(() => {
	if (volumePercent.value === 0) {
		return 'audio-volume-muted-symbolic';
	}
	return volumePercent.value < 50 ? 'audio-volume-low-symbolic' : 'audio-volume-high-symbolic';
});

const onInput = (e: Event) => {
	const target = e.target as HTMLInputElement;
	const val = Number(target.value);
	const normalized = Math.max(0, Math.min(100, val)) / 50;
	void playerStore.setVolume(normalized);
};
</script>

<template>
	<div class="hidden items-center gap-2 md:flex">
		<span class="inline-flex items-center gap-2 rounded-corner border border-ui-border bg-ui-surface/55 px-2 py-1 text-tx-main text-xs">
			<ThemeIcon :name="volumeIcon" type="symbol" :size="16" />
			<span class="text-tx-muted text-xs">{{ volumePercent }}%</span>
		</span>
		<input
			type="range"
			min="0"
			max="100"
			:value="volumePercent"
			:aria-label="t('volume.label')"
			:aria-valuetext="`${volumePercent}%`"
			class="h-1 w-36 appearance-none rounded bg-ui-border/40 accent-primary/70"
			@input="onInput"
		/>
	</div>
</template>

<style scoped>
input[type="range"]::-webkit-slider-thumb {
	-webkit-appearance: none;
	width: 14px;
	height: 14px;
	border-radius: 99px;
	background: var(--color-primary);
	box-shadow: 0 0 0 3px rgba(0, 0, 0, 0.05);
}
</style>
