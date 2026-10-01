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
 * ── Por qué es `Slider` y no `SliderControl` ───────────────────────────────
 *
 * `SliderControl` pide el icono como ruta ya resuelta y viene en un solo
 * tamaño, pensado para una fila de preferencias (vue-libvasak#52). `Slider`,
 * de la 2.1, es el control solo: se le da el ancho de acá (`w-36`) y trae el
 * contrato de accesibilidad —el nombre y «47 %» en `aria-valuetext`—.
 *
 * Se ve cuando la barra tiene lugar, mirando su propio ancho
 * (`@container` en `NowPlayingTopBar`) y no el de la pantalla.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Badge, Slider, ThemeIcon } from '@vasakgroup/vue-libvasak';
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

const onInput = (value: number) => {
	const normalized = Math.max(0, Math.min(100, value)) / 50;
	void playerStore.setVolume(normalized);
};
</script>

<template>
	<div class="hidden min-w-0 items-center gap-2 @min-[28rem]:flex">
		<Badge size="md">
			<span class="inline-flex items-center gap-2">
				<ThemeIcon :name="volumeIcon" type="symbol" :size="16" />
				<span class="text-tx-muted tabular-nums">{{ volumePercent }}%</span>
			</span>
		</Badge>
		<!-- Mismo criterio que el de la canción: 32 de zona de toque, y la fila
		     de los 26 píxeles que tenía. -->
		<Slider
			class="-my-[3px] w-36"
			:model-value="volumePercent"
			:min="0"
			:max="100"
			:label="t('volume.label')"
			:value-text="`${volumePercent}%`"
			@update:model-value="onInput"
		/>
	</div>
</template>
