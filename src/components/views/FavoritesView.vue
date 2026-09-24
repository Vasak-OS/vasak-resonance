<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { EmptyState, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import LabeledField from '@/components/layout/LabeledField.vue';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const { artistLabel, albumLabel } = useMetadataLabels();
const playerStore = usePlayerStore();
const { onTrackContextMenu } = useTrackContextMenu();
/**
 * Lo que mide una fila de favoritos, contando el hueco de abajo.
 *
 * El scroller coloca las filas él a partir de este número, así que tiene que
 * coincidir con lo que el CSS dibuja: 72 de alto más los 8 del `mb-2`. Si se
 * separan, las filas se pisan o dejan huecos, y no falla nada — se ve torcido.
 * Hay una prueba que compara los dos.
 */
const ALTO_DE_LA_FILA = 80;

const searchQuery = ref('');
const artistFilter = ref('all');
const sortBy = ref('recent');

const normalize = (value: string) => value.trim().toLowerCase();

const currentPath = computed(() => playerStore.currentPath || '');

const extractTrackName = (path: string): string => {
	const normalized = path.replace(/\\/g, '/');
	const parts = normalized.split('/');
	return parts[parts.length - 1] || path;
};

const filteredFavoriteEntries = computed(() => {
	const query = normalize(searchQuery.value);
	const base = playerStore.favoriteEntries.filter((entry) => {
		const artist = entry.metadata?.artist || 'Unknown Artist';
		if (artistFilter.value !== 'all' && artist !== artistFilter.value) {
			return false;
		}

		if (!query) {
			return true;
		}

		const title = entry.metadata?.title || extractTrackName(entry.path);
		const album = entry.metadata?.album || 'Unknown Album';
		return (
			normalize(title).includes(query) ||
			normalize(artist).includes(query) ||
			normalize(album).includes(query) ||
			normalize(entry.path).includes(query)
		);
	});

	if (sortBy.value === 'title-asc') {
		return [...base].sort((left, right) => {
			const leftTitle = left.metadata?.title || extractTrackName(left.path);
			const rightTitle = right.metadata?.title || extractTrackName(right.path);
			return leftTitle.localeCompare(rightTitle);
		});
	}

	if (sortBy.value === 'artist-asc') {
		return [...base].sort((left, right) => {
			const leftArtist = left.metadata?.artist || 'Unknown Artist';
			const rightArtist = right.metadata?.artist || 'Unknown Artist';
			return leftArtist.localeCompare(rightArtist);
		});
	}

	return base;
});

const favoriteArtistOptions = computed(() => {
	const values = new Set(
		playerStore.favoriteEntries.map((entry) => entry.metadata?.artist || 'Unknown Artist')
	);
	return Array.from(values).sort((left, right) => left.localeCompare(right));
});

onMounted(async () => {
	await playerStore.ensureMetadataForFavorites();
});
</script>

<template>
	<section class="flex h-full flex-col gap-3 overflow-hidden p-4">
		<div class="mb-4 flex flex-wrap items-center justify-between gap-3">
			<div>
				<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">{{ t('favorites.eyebrow') }}</p>
				<h2 class="text-lg font-semibold text-tx-main">{{ t('favorites.title') }}</h2>
			</div>
			<button
				type="button"
				class="inline-flex items-center gap-1 rounded-corner border border-primary/45 bg-primary px-3 py-2 text-xs font-semibold text-tx-on-primary transition-colors duration-200 hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-50"
				:disabled="!playerStore.hasTrack"
				:title="playerStore.isCurrentFavorite ? t('favorites.removeCurrent') : t('common.addFavorite')"
				:aria-label="playerStore.isCurrentFavorite ? t('favorites.removeCurrent') : t('common.addFavorite')"
				@click="playerStore.toggleCurrentFavorite"
			>
				<ThemeIcon
					:name="playerStore.isCurrentFavorite ? 'remove' : 'new-star'"
					type="symbol"
					:size="16"
				/>
				{{ playerStore.isCurrentFavorite ? t('favorites.removeCurrent') : t('favorites.saveCurrent') }}
			</button>
		</div>

		<div class="mb-4 grid gap-3 lg:grid-cols-[1.4fr_0.8fr_0.8fr]">
			<LabeledField :label="t('common.search')">
				<input
					v-model="searchQuery"
					type="search"
					:placeholder="t('home.searchPlaceholder')"
					class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 placeholder:text-tx-muted/70 focus:border-primary/50"
				/>
			</LabeledField>

			<LabeledField :label="t('common.artist')">
				<select v-model="artistFilter" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 focus:border-primary/50">
					<option value="all">{{ t('common.all') }}</option>
					<option v-for="artist in favoriteArtistOptions" :key="artist" :value="artist">{{ artistLabel(artist) }}</option>
				</select>
			</LabeledField>

			<LabeledField :label="t('common.sortBy')">
				<select v-model="sortBy" class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-2 text-sm text-tx-main transition-colors duration-200 focus:border-primary/50">
					<option value="recent">{{ t('sort.recent') }}</option>
					<option value="title-asc">{{ t('sort.titleAsc') }}</option>
					<option value="artist-asc">{{ t('sort.artistAsc') }}</option>
				</select>
			</LabeledField>
		</div>

		<EmptyState v-if="filteredFavoriteEntries.length === 0" :title="t('favorites.empty')" bordered />

		<!--
			Una fila por favorito, con alto fijo: es lo que el `RecycleScroller`
			necesita saber para colocar sin medir. `ALTO_DE_LA_FILA` la declara
			una sola vez y la prueba lo comprueba contra la clase, porque si los
			dos números se separan las filas se pisan o dejan huecos.
		-->
		<div v-else class="min-h-0 flex-1 overflow-hidden" @contextmenu="onTrackContextMenu">
			<RecycleScroller
				:items="filteredFavoriteEntries"
				key-field="path"
				:item-size="ALTO_DE_LA_FILA"
				class="h-full overflow-y-auto"
				v-slot="{ item: entry }"
			>
			<div
				:data-track-path="entry.path"
				class="mb-2 flex h-[72px] items-center gap-3 rounded-corner border border-ui-border bg-ui-bg/70 px-3 py-2"
			>
				<div class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-corner border border-ui-border bg-ui-surface/45">
					<img
						v-if="entry.metadata?.cover_data_url"
						:src="entry.metadata.cover_data_url"
						:alt="entry.metadata.title || extractTrackName(entry.path)"
						class="h-full w-full object-cover"
					/>
					<div v-else class="text-[10px] font-semibold uppercase tracking-[0.14em] text-tx-muted">{{ t('favorites.coverPlaceholder') }}</div>
				</div>

				<div class="min-w-0 flex-1">
					<p class="truncate text-sm font-medium text-tx-main">
						{{ entry.metadata?.title || extractTrackName(entry.path) }}
					</p>
					<p class="truncate text-xs text-tx-muted">
						{{ artistLabel(entry.metadata?.artist) }} • {{ albumLabel(entry.metadata?.album) }}
					</p>
					<p class="truncate text-[11px] text-tx-muted/80">{{ entry.path }}</p>
				</div>
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					:title="t('common.play')"
					:aria-label="t('common.play')"
					@click="playerStore.playDropped(entry.path)"
				>
					<ThemeIcon name="media-playback-start" type="symbol" :size="16" />
					{{ t('common.play') }}
				</button>
				<button
					type="button"
					class="inline-flex items-center gap-1 rounded-corner border border-status-error/35 bg-status-error/10 px-3 py-1.5 text-xs font-medium text-status-error transition-colors duration-200 hover:bg-status-error/20"
					:title="t('common.remove')"
					:aria-label="t('common.remove')"
					@click="playerStore.toggleFavoritePath(entry.path)"
				>
					<ThemeIcon name="remove" type="symbol" :size="16" />
					{{ t('common.remove') }}
				</button>
			</div>
			</RecycleScroller>
		</div>

		<p v-if="currentPath" class="mt-3 text-xs text-tx-muted">
			{{ t('favorites.current').replace('{0}', () => extractTrackName(currentPath)) }}
		</p>
	</section>
</template>
