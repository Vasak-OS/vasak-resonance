<script setup lang="ts">
import PlayerQueuePanel from '@/components/player/PlayerQueuePanel.vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Playlists</p>
			<h2 class="text-lg font-semibold text-tx-main">Gestion de cola activa</h2>
		</div>

		<PlayerQueuePanel
			v-if="playerStore.queue.length > 0"
			:queue-items="playerStore.queue"
			@clear="playerStore.clearQueue"
			@remove="playerStore.removeQueueItem"
			@reorder="playerStore.moveQueueItem"
		/>

		<div v-else class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			No hay pistas en cola. Arrastra varias canciones para crear una playlist temporal.
		</div>
	</section>
</template>
