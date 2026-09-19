<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { WindowFrame } from '@vasakgroup/vue-libvasak';
import { onMounted, onUnmounted } from 'vue';
import { RouterView } from 'vue-router';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import ResonanceSidebar from '@/components/layout/ResonanceSidebar.vue';
import NowPlayingTopBar from '@/components/player/NowPlayingTopBar.vue';
import PlayerBackground from '@/components/player/PlayerBackground.vue';
import { useAudioDrop } from '@/composables/useAudioDrop';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
const appIcon = useReactiveIcon('music-app', 'icon');
const miniIcon = useReactiveIcon('screenshot-ui-window');

useAudioDrop({
	onFilesDropped: (paths: string[]) => playerStore.handleDroppedPaths(paths),
	onDragStateChange: (dragging: boolean) => playerStore.setDragOver(dragging),
});

onMounted(async () => {
	await playerStore.initProgressListener();
	await playerStore.initMprisNextListener();
	await playerStore.initMprisPreviousListener();
	await playerStore.initMprisStopListener();
});

/**
 * Cerrar acá no es cerrar la ventana y ya.
 *
 * El audio se apaga primero: sin eso, el proceso se va con el dispositivo
 * tomado y el sonido queda cortado a mitad de una nota hasta que el sistema
 * limpia. Por eso el marco compartido deja reemplazar lo que hace el botón: si
 * alguien escucha `close`, `WindowControls` emite y **no** toca la ventana.
 *
 * Si el comando falla se cierra igual: una ventana que no se puede cerrar es
 * peor que un audio mal apagado.
 */
async function cerrar() {
	try {
		await invoke('close_app');
	} catch (error) {
		console.error('No se pudo apagar el audio antes de cerrar', error);
		await getCurrentWindow().close();
	}
}

onUnmounted(() => {
	playerStore.disposeProgressListener();
	playerStore.disposeMprisNextListener();
	playerStore.disposeMprisPreviousListener();
	playerStore.disposeMprisStopListener();
});
</script>
<template>
	<WindowFrame
		:minimize-label="t('windowControls.minimize')"
		:maximize-label="t('windowControls.maximize')"
		:close-label="t('windowControls.close')"
		@close="cerrar()"
	>
		<template #identidad>
			<img :src="appIcon" class="h-8 w-8" :alt="t('window.appIconAlt')">
		</template>

		<!-- El nombre al medio de la ventana entera. Estaba centrado con un tercer
		     `div` vacío tirando contra el `justify-between` de la barra propia, y
		     eso lo deja centrado respecto de lo que sobra entre el icono y los
		     controles, que ocupan bastante más. -->
		<template #centro>
			<span class="font-title font-semibold text-lg">Resonance</span>
		</template>

		<!-- Pasar al mini reproductor, junto a los botones de la ventana: es una
		     acción sobre la ventana, no sobre lo que está sonando. -->
		<template #acciones>
			<button
				type="button"
				class="rounded-corner border border-ui-border bg-ui-bg/80 px-2 py-1 font-medium text-primary text-xs hover:bg-primary/20"
				:title="t('windowControls.miniPlayer')"
				:aria-label="t('windowControls.miniPlayer')"
				@click="toggleMainAndMiniPlayer()"
			>
				<img :src="miniIcon" class="inline-block h-4 w-4" alt="">
			</button>
		</template>

		<main class="relative flex-1 overflow-hidden p-1 pt-0">
			<PlayerBackground />

			<AudioDropOverlay :is-active="playerStore.isDragOver" />

			<Transition
				enter-active-class="transition-[opacity,translate] duration-200 ease-out"
				enter-from-class="opacity-0 -translate-y-2"
				leave-active-class="transition-[opacity,translate] duration-150 ease-in"
				leave-to-class="opacity-0 -translate-y-2"
			>
				<div
					v-if="playerStore.globalBadgeMessage"
					class="pointer-events-none absolute left-1/2 top-2 z-30 -translate-x-1/2 rounded-corner border border-primary/35 bg-primary/12 px-3 py-2 text-xs font-medium text-primary backdrop-blur-sm"
				>
					{{ playerStore.globalBadgeMessage }}
				</div>
			</Transition>

			<div class="relative z-10 flex h-full w-full flex-col gap-3 md:flex-row">
				<ResonanceSidebar />

				<div class="min-h-0 min-w-0 flex h-full flex-1 flex-col gap-2 overflow-hidden">
					<div class="min-h-0 flex-1 overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80">
						<RouterView v-slot="{ Component }">
							<component :is="Component" class="h-full w-full" />
						</RouterView>
					</div>
					<NowPlayingTopBar class="shrink-0" />
				</div>
			</div>
		</main>
	</WindowFrame>
</template>
