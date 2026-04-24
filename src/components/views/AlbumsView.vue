<script setup lang="ts">
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const albumName = computed(() => playerStore.currentTrack?.album || 'Sin album detectado');
const artistName = computed(() => playerStore.currentTrack?.artist || 'Unknown Artist');
const coverArt = computed(() => playerStore.currentTrack?.cover_data_url || '');
const trackCountLabel = computed(() =>
	playerStore.hasTrack ? '1 pista activa' : '0 pistas activas'
);
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Albums</p>
			<h2 class="text-lg font-semibold text-tx-main">Biblioteca por album</h2>
		</div>

		<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
			<article class="rounded-corner border border-ui-border bg-ui-bg/80 p-4">
				<div class="mb-3 flex h-44 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
					<img v-if="coverArt" :src="coverArt" :alt="albumName" class="h-full w-full object-cover" />
					<div v-else class="text-sm font-semibold uppercase tracking-[0.16em] text-tx-muted">No Cover</div>
				</div>
				<p class="truncate text-base font-semibold text-tx-main">{{ albumName }}</p>
				<p class="truncate text-sm text-tx-muted">{{ artistName }}</p>
				<p class="mt-2 text-xs uppercase tracking-[0.12em] text-primary">{{ trackCountLabel }}</p>
			</article>
		</div>
	</section>
</template>
