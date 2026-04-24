<script setup lang="ts">
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const currentPath = computed(() => playerStore.currentPath || '');

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};
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
				class="rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
				:disabled="!playerStore.hasTrack"
				@click="playerStore.toggleCurrentFavorite"
			>
				{{ playerStore.isCurrentFavorite ? 'Quitar actual' : 'Guardar actual' }}
			</button>
		</div>

		<div v-if="playerStore.favoritePaths.length === 0" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			Aun no tienes canciones en favoritos.
		</div>

		<ul v-else class="grid gap-2">
			<li
				v-for="path in playerStore.favoritePaths"
				:key="path"
				class="flex items-center gap-3 rounded-corner border border-ui-border bg-ui-bg/70 px-3 py-2"
			>
				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-medium text-tx-main">{{ extractTrackName(path) }}</p>
					<p class="truncate text-xs text-tx-muted">{{ path }}</p>
				</div>
				<button
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					@click="playerStore.playDropped(path)"
				>
					Reproducir
				</button>
				<button
					type="button"
					class="rounded-corner border border-status-error/35 bg-status-error/10 px-3 py-1.5 text-xs font-medium text-status-error transition-colors duration-200 hover:bg-status-error/20"
					@click="playerStore.toggleFavoritePath(path)"
				>
					Quitar
				</button>
			</li>
		</ul>

		<p v-if="currentPath" class="mt-3 text-xs text-tx-muted">
			Actual: {{ extractTrackName(currentPath) }}
		</p>
	</section>
</template>
