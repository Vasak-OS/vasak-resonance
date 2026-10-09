<script setup lang="ts">
/**
 * La tapa de lo que suena con su título y su artista.
 *
 * La tapa es `CoverArt` de la librería: cuadrada, con el canto y el radio del
 * sistema, y con el respaldo —el texto, o el icono del tema— cuando no hay
 * imagen o la que hay no carga. Antes era una caja de 220 × 144 que recortaba
 * la tapa, que es cuadrada, por arriba y por abajo.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { CoverArt } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';

const props = withDefaults(
	defineProps<{
		title: string;
		subtitle: string;
		coverSrc?: string | null;
		variant?: 'compact' | 'stacked';
		placeholderText?: string;
		titleClass?: string;
		subtitleClass?: string;
	}>(),
	{
		coverSrc: null,
		variant: 'compact',
		// Vacío y no el texto de reserva: el valor por omisión se evalúa fuera de
		// `setup`, donde `t()` todavía no existe. Se resuelve más abajo.
		placeholderText: '',
		titleClass: 'text-tx-main',
		subtitleClass: 'text-tx-muted',
	}
);

const { t } = useI18n();

const placeholder = computed(() => props.placeholderText || t('common.noCover'));

const rootClass = computed(() => {
	return props.variant === 'stacked'
		? 'flex flex-col items-center'
		: 'flex min-h-0 flex-1 items-center gap-3';
});

const metaClass = computed(() => {
	return props.variant === 'stacked' ? 'mb-3 w-full space-y-1' : 'min-w-0 flex-1';
});
</script>

<template>
	<div :class="rootClass">
		<!-- Apilada mide lo mismo que antes de alto (`h-36`), ahora cuadrada. -->
		<div v-if="variant === 'stacked'" class="mx-auto mb-3 aspect-square h-36 max-w-full">
			<CoverArt :src="coverSrc" :alt="title" :fallback-text="placeholder" size="fill" />
		</div>
		<CoverArt v-else :src="coverSrc" :alt="title" :fallback-text="placeholder" size="md" />

		<div :class="metaClass">
			<p class="truncate text-sm font-semibold" :class="titleClass">{{ title }}</p>
			<p class="truncate text-xs" :class="subtitleClass">{{ subtitle }}</p>
		</div>
	</div>
</template>
