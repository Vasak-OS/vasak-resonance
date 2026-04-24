<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();

const groupedAlbums = computed(() => {
	const albumsMap = new Map<
		string,
		{
			key: string;
			album: string;
			artist: string;
			cover: string;
			tracks: { path: string; title: string; artist: string }[];
		}
	>();

	for (const track of playerStore.trackCacheList) {
		const album = track.album || 'Unknown Album';
		const key = album.toLowerCase();
		if (!albumsMap.has(key)) {
			albumsMap.set(key, {
				key,
				album,
				artist: track.artist || 'Unknown Artist',
				cover: track.cover_data_url || '',
				tracks: [],
			});
		}

		const group = albumsMap.get(key);
		if (!group) {
			continue;
		}

		if (!group.cover && track.cover_data_url) {
			group.cover = track.cover_data_url;
		}

		group.tracks.push({
			path: track.path,
			title: track.title || 'Unknown track',
			artist: track.artist || 'Unknown Artist',
		});
	}

	return Array.from(albumsMap.values()).sort((a, b) => a.album.localeCompare(b.album));
});

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

onMounted(async () => {
	await playerStore.ensureMetadataForFavorites();
});

const onPlayTrack = async (path: string) => {
	await playerStore.playDropped(path);
};

const onQueueAlbum = (paths: string[]) => {
	playerStore.enqueuePaths(paths);
};
</script>

<template>
	<section class="h-full overflow-y-auto p-4">
		<div class="mb-4">
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">Albums</p>
			<h2 class="text-lg font-semibold text-tx-main">Biblioteca por album</h2>
		</div>

		<div v-if="groupedAlbums.length === 0" class="rounded-corner border border-dashed border-ui-border bg-ui-surface/35 p-4 text-sm text-tx-muted">
			No hay albumes en cache todavia. Reproduce o marca canciones como favoritas para construir la biblioteca.
		</div>

		<div v-else class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
			<article
				v-for="album in groupedAlbums"
				:key="album.key"
				class="rounded-corner border border-ui-border bg-ui-bg/80 p-4"
			>
				<div class="mb-3 flex h-44 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
					<img v-if="album.cover" :src="album.cover" :alt="album.album" class="h-full w-full object-cover" />
					<div v-else class="text-sm font-semibold uppercase tracking-[0.16em] text-tx-muted">No Cover</div>
				</div>
				<p class="truncate text-base font-semibold text-tx-main">{{ album.album }}</p>
				<p class="truncate text-sm text-tx-muted">{{ album.artist }}</p>
				<p class="mt-2 text-xs uppercase tracking-[0.12em] text-primary">
					{{ album.tracks.length }} {{ album.tracks.length === 1 ? 'pista' : 'pistas' }}
				</p>

				<div class="mt-3">
					<button
						type="button"
						class="w-full rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-xs font-semibold text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
						@click="onQueueAlbum(album.tracks.map((track) => track.path))"
					>
						Encolar album
					</button>
				</div>

				<ul class="mt-3 grid gap-1.5">
					<li
						v-for="track in album.tracks.slice(0, 4)"
						:key="track.path"
						class="flex items-center justify-between gap-2 rounded-corner border border-transparent px-2 py-1 hover:border-ui-border hover:bg-ui-surface/45"
					>
						<span class="min-w-0 flex-1 truncate text-xs text-tx-muted">
							{{ track.title || extractTrackName(track.path) }}
						</span>
						<button
							type="button"
							class="shrink-0 rounded-corner border border-primary/45 bg-primary/10 px-2 py-1 text-[11px] font-medium text-primary transition-colors duration-200 hover:bg-primary/20"
							@click="onPlayTrack(track.path)"
						>
							Reproducir
						</button>
					</li>
				</ul>
			</article>
		</div>
	</section>
</template>
