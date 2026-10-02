<script setup lang="ts">
/**
 * El aviso de «buscando música» mientras se recorre la carpeta.
 *
 * Es un `Dialog` de la librería y no una capa propia: el velo, la superficie
 * que flota y el foco atrapado salen de ahí. La capa de antes usaba
 * `bg-bg-primary`, un color que no existe —la caja quedaba transparente sobre
 * el velo—, y un negro fijo con desenfoque detrás.
 *
 * No se puede cerrar: el recorrido termina solo, y cerrar el aviso no lo
 * cancelaría. Por eso `open` va sin `v-model`: ni Escape ni un clic en el velo
 * lo cambian, sólo el almacén cuando termina el recorrido.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Dialog, DialogContent, LoadingState } from '@vasakgroup/vue-libvasak';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
</script>

<template>
	<Dialog :open="playerStore.isScanning">
		<DialogContent size="sm" :ariaLabel="t('scanning.title')">
			<div class="flex flex-col items-center gap-2 p-4">
				<LoadingState :label="t('scanning.title')" />
				<p class="text-center text-xs text-tx-muted">{{ t('scanning.subtitle') }}</p>
			</div>
		</DialogContent>
	</Dialog>
</template>
