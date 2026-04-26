	<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted } from 'vue';
import { ref } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
const playIcon = ref('');
const addFavoriteIcon = ref('');
const removeIcon = ref('');

const currentPath = computed(() => playerStore.currentPath || '');

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

onMounted(async () => {
	const getSymbolic = getSymbolSource;
	const [playSrc, addFavSrc, removeSrc] = await Promise.all([
		getSymbolic('media-playback-start').catch(() => ''),
		getSymbolic('new-star').catch(() => ''),
		getSymbolic('remove').catch(() => ''),
	]);

	playIcon.value = playSrc;
	addFavoriteIcon.value = addFavSrc;
	removeIcon.value = removeSrc;

	await playerStore.ensureMetadataForFavorites();
});
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4 flex flex-wrap items-center justify-between gap-3">
			<div>
				<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Favoritos</p>
				<h2 class="text-lg font-semibold text-tx-main">Tus canciones guardadas</h2>
			</div>
			<button
				type="button"
				class="inline-flex items-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
				:disabled="!playerStore.hasTrack"
				:title="playerStore.isCurrentFavorite ? 'Quitar actual' : 'Agregar a favorito'"
				:aria-label="playerStore.isCurrentFavorite ? 'Quitar actual' : 'Agregar a favorito'"
				@click="playerStore.toggleCurrentFavorite"
			>
				<img
					v-if="playerStore.isCurrentFavorite ? removeIcon : addFavoriteIcon"
					:src="playerStore.isCurrentFavorite ? removeIcon : addFavoriteIcon"
					:alt="playerStore.isCurrentFavorite ? 'Quitar actual' : 'Agregar a favorito'"
					class="h-4 w-4"
				>
				{{ playerStore.isCurrentFavorite ? 'Quitar actual' : 'Guardar actual' }}
			</button>
		</div>

		<div v-if="playerStore.favoritePaths.length === 0" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			Aun no tienes canciones en favoritos.
		</div>

		<ul v-else class="grid gap-2">
			<li
				v-for="entry in playerStore.favoriteEntries"
				:key="entry.path"
				class="flex items-center gap-3 rounded-corner border border-ui-border bg-ui-bg/70 px-3 py-2"
			>
				<div class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
					<img
						v-if="entry.metadata?.cover_data_url"
						:src="entry.metadata.cover_data_url"
						:alt="entry.metadata.title || extractTrackName(entry.path)"
						class="h-full w-full object-cover"
					/>
					<div v-else class="text-[10px] font-semibold uppercase tracking-[0.14em] text-tx-muted">Fav</div>
				</div>

				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-medium text-tx-main">
						{{ entry.metadata?.title || extractTrackName(entry.path) }}
					</p>
					<p class="truncate text-xs text-tx-muted">
						{{ entry.metadata?.artist || 'Unknown Artist' }} • {{ entry.metadata?.album || 'Unknown Album' }}
					</p>
					<p class="truncate text-[11px] text-tx-muted/80">{{ entry.path }}</p>
				</div>
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					title="Reproducir"
					aria-label="Reproducir"
					@click="playerStore.playDropped(entry.path)"
				>
					<img v-if="playIcon" :src="playIcon" alt="Reproducir" class="h-4 w-4">
					Reproducir
				</button>
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-status-error/35 bg-status-error/10 px-3 py-1.5 text-xs font-medium text-status-error transition-colors duration-200 hover:bg-status-error/20"
					title="Quitar"
					aria-label="Quitar"
					@click="playerStore.toggleFavoritePath(entry.path)"
				>
					<img v-if="removeIcon" :src="removeIcon" alt="Quitar" class="h-4 w-4">
					Quitar
				</button>
			</li>
		</ul>

		<p v-if="currentPath" class="mt-3 text-xs text-tx-muted">
			Actual: {{ extractTrackName(currentPath) }}
		</p>
	</section>
</template>
