<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import TrackMetaCard from '@/components/player/TrackMetaCard.vue';
import MainTransportControls from '@/components/player/transport/MainTransportControls.vue';
import { useTrackSubtitle } from '@/composables/useTrackSubtitle';
import { useTrackTitle } from '@/composables/useTrackTitle';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
const router = useRouter();
const route = useRoute();

const sections = [
	{ id: 'home', label: 'Inicio', icon: 'go-home-symbolic' },
	{ id: 'albums', label: 'Albums', icon: 'folder-music-symbolic' },
	{ id: 'favorites', label: 'Favoritos', icon: 'starred-symbolic' },
	{ id: 'playlists', label: 'Playlists', icon: 'view-list-symbolic' },
] as const;

const iconSources = ref<Record<string, string>>({});

const selectedSection = computed(() => (typeof route.name === 'string' ? route.name : 'home'));

const coverArt = computed(() => playerStore.currentTrack?.cover_data_url || '');

const trackTitle = useTrackTitle({
	currentTrack: () => playerStore.currentTrack,
	currentPath: () => playerStore.currentPath,
	fallback: 'Sin reproduccion',
});

const trackSubtitle = useTrackSubtitle({
	currentTrack: () => playerStore.currentTrack,
});

const onSelectSection = async (id: string) => {
	if (selectedSection.value === id) {
		return;
	}

	await router.push({ name: id });
};

onMounted(async () => {
	const loadedSources = await Promise.all(
		sections.map(async (section) => {
			try {
				const src = await getSymbolSource(section.icon);
				return [section.id, src] as const;
			} catch {
				return [section.id, ''] as const;
			}
		})
	);

	iconSources.value = Object.fromEntries(loadedSources);
});
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
				class="flex w-full items-center gap-3 rounded-corner border px-3 py-2 text-left text-sm transition-all duration-200"
				:class="[
					selectedSection === section.id
						? 'border-secondary bg-primary/15 text-tx-main'
						: 'border-transparent bg-ui-bg/30 text-tx-muted hover:border-ui-border hover:bg-ui-surface/70 hover:text-tx-main',
					'cursor-pointer',
				]"
				@click="onSelectSection(section.id)"
			>
				<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-corner border border-ui-border bg-ui-bg/55">
					<img
						v-if="iconSources[section.id]"
						:src="iconSources[section.id]"
						:alt="section.label"
						class="h-5 w-5 object-contain"
					/>
					<span v-else class="text-xs font-semibold">{{ section.label.charAt(0) }}</span>
				</span>
				<span class="flex-1">{{ section.label }}</span>
			</button>
		</nav>

		<section class="mt-2 rounded-corner border border-ui-border bg-ui-surface/40 p-3">
			<TrackMetaCard
				:title="trackTitle"
				:subtitle="trackSubtitle"
				:cover-src="coverArt"
				variant="stacked"
				placeholder-text="VR"
			/>

			<MainTransportControls
				:has-track="playerStore.hasTrack"
				:has-next-track="playerStore.hasNextTrack"
				:busy="playerStore.busy"
				:is-paused="playerStore.isPaused"
				:next-action-label="playerStore.nextActionLabel"
				@prev="playerStore.playPreviousTrack"
				@toggle="playerStore.togglePlayPause"
				@next="playerStore.advanceQueue"
			/>
		</section>
	</aside>
</template>