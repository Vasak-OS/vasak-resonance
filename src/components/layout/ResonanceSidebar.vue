<script setup lang="ts">
import { computed, ref } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const sections = [
	{ id: 'home', label: 'Inicio' },
	{ id: 'albums', label: 'Albums', disabled: true },
	{ id: 'favorites', label: 'Favoritos', disabled: true },
	{ id: 'playlists', label: 'Playlists', disabled: true },
];

const selectedSection = ref('home');

const coverArt = computed(() => playerStore.currentTrack?.cover_data_url || '');

const trackTitle = computed(() => {
	if (playerStore.currentTrack?.title) {
		return playerStore.currentTrack.title;
	}
	if (playerStore.currentPath) {
		const parts = playerStore.currentPath.split('/');
		return parts[parts.length - 1] || 'Unknown track';
	}
	return 'Sin reproducción';
});

const trackSubtitle = computed(() => {
	if (!playerStore.currentTrack) {
		return 'Vasak Resonance';
	}
	const artist = playerStore.currentTrack.artist || 'Unknown Artist';
	const album = playerStore.currentTrack.album || 'Unknown Album';
	return `${artist} • ${album}`;
});

const playButtonLabel = computed(() => {
	if (!playerStore.hasTrack) {
		return 'Play';
	}
	return playerStore.isPaused ? 'Play' : 'Pause';
});

const onSelectSection = (id: string) => {
	selectedSection.value = id;
};
</script>

<template>
	<aside class="flex w-full shrink-0 flex-col rounded-corner border border-ui-border bg-ui-bg/80 p-2 md:w-72">
		<header class="border-b border-ui-border px-2 pb-3 pt-1">
			<p class="text-xs uppercase tracking-[0.12em] text-tx-muted">Navegacion</p>
			<p class="text-sm font-semibold text-tx-main">Biblioteca</p>
		</header>

		<nav class="flex-1 space-y-2 overflow-y-auto px-1 py-3">
			<button
				v-for="section in sections"
				:key="section.id"
				type="button"
				:disabled="section.disabled"
				class="flex w-full items-center justify-between rounded-corner border px-3 py-2 text-left text-sm transition-all duration-200"
				:class="[
					selectedSection === section.id
						? 'border-secondary bg-primary/15 text-tx-main'
						: 'border-transparent bg-ui-bg/30 text-tx-muted hover:border-ui-border hover:bg-ui-surface/70 hover:text-tx-main',
					section.disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer',
				]"
				@click="onSelectSection(section.id)"
			>
				<span>{{ section.label }}</span>
				<span v-if="section.disabled" class="text-[10px] uppercase tracking-[0.14em] text-tx-muted/80">Soon</span>
			</button>
		</nav>

		<section class="mt-2 rounded-corner border border-ui-border bg-ui-surface/40 p-3">
			<div
				class="mx-auto mb-3 flex h-36 w-full max-w-[220px] items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-bg/60"
			>
				<img
					v-if="coverArt"
					:src="coverArt"
					:alt="trackTitle"
					class="h-full w-full object-cover"
				/>
				<div v-else class="text-sm font-semibold uppercase tracking-[0.16em] text-tx-muted">VR</div>
			</div>

			<div class="mb-3 space-y-1">
				<p class="truncate text-sm font-semibold text-tx-main">{{ trackTitle }}</p>
				<p class="truncate text-xs text-tx-muted">{{ trackSubtitle }}</p>
			</div>

			<div class="grid grid-cols-3 gap-2">
				<button
					type="button"
					class="rounded-corner border border-ui-border bg-ui-bg/50 px-2 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:bg-ui-surface/80 disabled:cursor-not-allowed disabled:opacity-50"
					:disabled="!playerStore.hasTrack || playerStore.busy"
					@click="playerStore.playPreviousTrack"
				>
					Prev
				</button>
				<button
					type="button"
					class="rounded-corner border border-primary/45 bg-primary px-2 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
					:disabled="!playerStore.hasTrack || playerStore.busy"
					@click="playerStore.togglePlayPause"
				>
					{{ playButtonLabel }}
				</button>
				<button
					type="button"
					class="rounded-corner border border-ui-border bg-ui-bg/50 px-2 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:bg-ui-surface/80 disabled:cursor-not-allowed disabled:opacity-50"
					:disabled="playerStore.queuedCount <= 0 || playerStore.busy"
					@click="playerStore.advanceQueue"
				>
					Next
				</button>
			</div>
		</section>
	</aside>
</template>