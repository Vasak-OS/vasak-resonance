<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	ThemeIcon,
	ToastArea,
	type ToastNotice,
	WindowFrame,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { RouterView, useRoute } from 'vue-router';
import AudioDropOverlay from '@/components/layout/AudioDropOverlay.vue';
import ResonanceSidebar from '@/components/layout/ResonanceSidebar.vue';
import NowPlayingTopBar from '@/components/player/NowPlayingTopBar.vue';
import PlayerBackground from '@/components/player/PlayerBackground.vue';
import { useAudioDrop } from '@/composables/useAudioDrop';
import { useNarrowRow } from '@/composables/useNarrowRow';
import { toggleMainAndMiniPlayer } from '@/services/window.service';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();
const route = useRoute();

useAudioDrop({
	onFilesDropped: (paths: string[]) => playerStore.handleDroppedPaths(paths),
	onDragStateChange: (dragging: boolean) => playerStore.setDragOver(dragging),
});

/**
 * Una columna por vez en ventana angosta, como en un teléfono.
 *
 * Por debajo del ancho donde caben las dos (`SIDE_BY_SIDE_MIN_WIDTH`), la barra
 * lateral deja de ir al costado del contenido: se abre **en lugar** del
 * contenido con el botón «Biblioteca» de la barra de la ventana, y elegir una
 * sección la vuelve a cerrar. Antes la barra se apilaba encima y, con sus siete
 * secciones y la tapa, se comía el alto entero: a 600 y a 360 de ancho el
 * contenido quedaba fuera de la ventana.
 */
const row = ref<HTMLElement | null>(null);
const narrow = useNarrowRow(row);
const libraryOpen = ref(false);

const showSidebar = computed(() => !narrow.value || libraryOpen.value);
const showContent = computed(() => !narrow.value || !libraryOpen.value);

// Al ensancharse la ventana las dos columnas vuelven a verse juntas; al
// volver a angostarse arranca por el contenido, que es lo que se estaba
// mirando.
watch(narrow, () => {
	libraryOpen.value = false;
});

// Cambiar de sección es lo que se vino a hacer a la barra: el contenido nuevo
// ocupa la ventana.
watch(
	// Sin enrutador —una prueba que monta sólo el marco— no hay ruta que mirar.
	() => route?.fullPath,
	() => {
		libraryOpen.value = false;
	}
);

const libraryLabel = computed(() =>
	libraryOpen.value ? t('window.hideLibrary') : t('window.showLibrary')
);

/** El aviso de la ventana —«agregado a favoritos», «en la cola»— como aviso de la librería. */
const toasts = computed<ToastNotice[]>(() =>
	playerStore.globalBadgeMessage
		? [
				{
					id: playerStore.globalBadgeMessage,
					message: playerStore.globalBadgeMessage,
					tone: 'info',
				},
			]
		: []
);

onMounted(async () => {
	await playerStore.initProgressListener();
	await playerStore.initMprisNextListener();
	await playerStore.initMprisPreviousListener();
	await playerStore.initMprisStopListener();
	await playerStore.initMprisModeListeners();
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
async function closeApp() {
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
		@close="closeApp()"
	>
		<template #identidad>
			<ThemeIcon name="music-app" :size="32" :alt="t('window.appIconAlt')" />
		</template>

		<!-- El nombre al medio de la ventana entera, encima de la barra. En
		     angosto no entra ahí sin pisar los botones, así que pasa al lugar
		     del título, que se recorta antes de empujar a nadie. -->
		<template v-if="!narrow" #centro>
			<span class="font-title font-semibold text-lg">Resonance</span>
		</template>
		<template v-else #titulo>
			<span class="font-title font-semibold">Resonance</span>
		</template>

		<!-- Pasar al mini reproductor, junto a los botones de la ventana: es una
		     acción sobre la ventana, no sobre lo que está sonando. En angosto,
		     antes, el botón que abre la biblioteca en lugar del contenido. -->
		<template #acciones>
			<ActionButton
				v-if="narrow"
				label=""
				:icon-alt="libraryLabel"
				:title="libraryLabel"
				icon="view-sidebar-symbolic"
				variant="ghost"
				:pressed="libraryOpen"
				@click="libraryOpen = !libraryOpen"
			/>
			<ActionButton
				label=""
				:icon-alt="t('windowControls.miniPlayer')"
				:title="t('windowControls.miniPlayer')"
				icon="screenshot-ui-window"
				variant="secondary"
				@click="toggleMainAndMiniPlayer()"
			/>
		</template>

		<main class="relative min-w-0 flex-1 overflow-hidden p-1 pt-0">
			<PlayerBackground />

			<AudioDropOverlay :is-active="playerStore.isDragOver" />

			<ToastArea :toasts="toasts" position="bottom-center" />

			<div
				ref="row"
				class="relative z-10 flex h-full w-full gap-3"
				:data-narrow="narrow || undefined"
			>
				<ResonanceSidebar v-show="showSidebar" :fill="narrow" />

				<div
					v-show="showContent"
					class="flex h-full min-h-0 min-w-0 flex-1 flex-col gap-2 overflow-hidden"
				>
					<div class="min-h-0 flex-1 overflow-hidden rounded-corner-m border border-ui-line bg-ui-surface/70">
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
