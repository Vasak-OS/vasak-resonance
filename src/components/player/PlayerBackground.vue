<script setup lang="ts">
/**
 * El degradado de abajo, teñido con el color de la tapa que suena.
 *
 * El color es un **dato**, no un color de la interfaz: lo calcula Rust con los
 * bytes de la tapa (`dominant_color`). Es la excepción de la §5 del inventario
 * de vue-libvasak#74 y lo único de la ventana que no sale del esquema. Se
 * mezcla con `color-mix()` contra transparente, que da el mismo color con la
 * opacidad pedida: antes se partía el hexadecimal a mano para armar un
 * `rgba()`. Sin tapa, el degradado sale del primario y el secundario del
 * esquema.
 */
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

/** Sólo un `#rrggbb`: cualquier otra cosa que llegue no entra en el estilo. */
const normalizeHex = (color: string | null | undefined): string | null => {
	const value = color?.trim();
	return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : null;
};

const gradientStyle = computed(() => {
	const dominant = normalizeHex(playerStore.currentTrack?.dominant_color);
	if (!dominant) {
		return {
			background:
				'linear-gradient(to top, color-mix(in srgb, var(--color-primary) 26%, transparent) 0%, color-mix(in srgb, var(--color-secondary) 18%, transparent) 48%, transparent 100%)',
		};
	}

	return {
		background: `linear-gradient(to top, color-mix(in srgb, ${dominant} 28%, transparent) 0%, color-mix(in srgb, ${dominant} 16%, transparent) 48%, color-mix(in srgb, ${dominant} 5%, transparent) 80%, transparent 100%)`,
	};
});
</script>

<template>
	<div class="pointer-events-none absolute inset-0 z-0 overflow-hidden rounded-corner-m" :style="gradientStyle" />
</template>
