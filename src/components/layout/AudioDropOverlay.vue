<script setup lang="ts">
/**
 * El aviso de «soltá acá» mientras se arrastran archivos sobre la ventana.
 *
 * Es `DropZone` de la librería en su forma de capa: el velo del esquema, el
 * recuadro punteado y el texto, que además se anuncia en voz alta. Antes era un
 * recuadro propio con el mismo dibujo y sin anunciarse.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { DropZone } from '@vasakgroup/vue-libvasak';

defineProps<{
	isActive: boolean;
	message?: string;
}>();

const { t } = useI18n();
</script>

<template>
	<DropZone overlay :active="isActive" :label="message || t('audioDrop.hint')" icon="audio-x-generic" />
</template>
