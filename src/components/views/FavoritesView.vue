<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	CoverArt,
	EmptyState,
	FormGroup,
	ListRow,
	PageHeader,
	SearchField,
	SelectField,
	type SelectOption,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useElementWidth } from '@/composables/useElementWidth';
import { useMetadataLabels } from '@/composables/useMetadataLabels';
import { useTrackContextMenu } from '@/composables/useTrackContextMenu';
import { usePlayerStore } from '@/stores/player';
import { rowActionsWithLabels } from '@/tools/window-layout';

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
const ROW_HEIGHT = 80;

/** La lista, para saber si los botones de cada fila entran con su texto. */
const list = ref<HTMLElement | null>(null);
const listWidth = useElementWidth(list);
const labeledActions = computed(() => rowActionsWithLabels(listWidth.value));

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

const artistSelectOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('common.all'), value: 'all' },
	...favoriteArtistOptions.value.map((artist) => ({ label: artistLabel(artist), value: artist })),
]);

const sortOptions = computed<SelectOption<string>[]>(() => [
	{ label: t('sort.recent'), value: 'recent' },
	{ label: t('sort.titleAsc'), value: 'title-asc' },
	{ label: t('sort.artistAsc'), value: 'artist-asc' },
]);

const currentFavoriteLabel = computed(() =>
	playerStore.isCurrentFavorite ? t('favorites.removeCurrent') : t('common.addFavorite')
);

onMounted(async () => {
	await playerStore.ensureMetadataForFavorites();
});
</script>

<template>
	<!-- `@container`: los filtros se ponen en fila cuando la vista tiene lugar
	     (antes `lg:`, 1024 de ventana; 44 rem de vista es el mismo corte). -->
	<section class="@container flex h-full flex-col gap-3 overflow-hidden p-4">
		<PageHeader class="mb-4" :eyebrow="t('favorites.eyebrow')" :title="t('favorites.title')" as="h2">
			<template #actions>
				<ActionButton
					:label="playerStore.isCurrentFavorite ? t('favorites.removeCurrent') : t('favorites.saveCurrent')"
					:title="currentFavoriteLabel"
					:icon="playerStore.isCurrentFavorite ? 'remove' : 'new-star'"
					:disabled="!playerStore.hasTrack"
					@click="playerStore.toggleCurrentFavorite"
				/>
			</template>
		</PageHeader>

		<div class="mb-4 grid gap-3 @min-[44rem]:grid-cols-[1.4fr_0.8fr_0.8fr]">
			<FormGroup :label="t('common.search')" variant="eyebrow">
				<SearchField v-model="searchQuery" :label="t('common.search')" :placeholder="t('home.searchPlaceholder')" />
			</FormGroup>

			<FormGroup v-slot="{ id }" :label="t('common.artist')" variant="eyebrow">
				<SelectField v-bind="{ id }" v-model="artistFilter" :options="artistSelectOptions" />
			</FormGroup>

			<FormGroup v-slot="{ id }" :label="t('common.sortBy')" variant="eyebrow">
				<SelectField v-bind="{ id }" v-model="sortBy" :options="sortOptions" />
			</FormGroup>
		</div>

		<EmptyState v-if="filteredFavoriteEntries.length === 0" :title="t('favorites.empty')" bordered />

		<!--
			Una fila por favorito, con alto fijo: es lo que el `RecycleScroller`
			necesita saber para colocar sin medir. `ROW_HEIGHT` la declara una
			sola vez y la prueba lo comprueba contra la clase, porque si los dos
			números se separan las filas se pisan o dejan huecos.
		-->
		<div v-else ref="list" class="min-h-0 flex-1 overflow-hidden" @contextmenu="onTrackContextMenu">
			<RecycleScroller
				:items="filteredFavoriteEntries"
				key-field="path"
				:item-size="ROW_HEIGHT"
				class="h-full overflow-y-auto"
				v-slot="{ item: entry }"
			>
				<div :data-track-path="entry.path" class="mb-2 h-[72px]">
					<ListRow class="h-full border border-ui-line bg-ui-surface/70">
						<template #leading>
							<span class="size-12">
								<CoverArt
									:src="entry.metadata?.cover_data_url ?? null"
									:alt="entry.metadata?.title || extractTrackName(entry.path)"
									:fallback-text="t('favorites.coverPlaceholder')"
								/>
							</span>
						</template>

						<span class="truncate text-label-m">{{ entry.metadata?.title || extractTrackName(entry.path) }}</span>
						<span class="truncate text-xs text-tx-muted">
							{{ artistLabel(entry.metadata?.artist) }} • {{ albumLabel(entry.metadata?.album) }}
						</span>
						<span class="truncate text-[11px] text-tx-muted">{{ entry.path }}</span>

						<template #trailing>
							<ActionButton
								:label="labeledActions ? t('common.play') : ''"
								:icon-alt="t('common.play')"
								:title="t('common.play')"
								icon="media-playback-start"
								variant="secondary"
								@click="playerStore.playDropped(entry.path)"
							/>
							<ActionButton
								:label="labeledActions ? t('common.remove') : ''"
								:icon-alt="t('common.remove')"
								:title="t('common.remove')"
								icon="remove"
								variant="secondary"
								@click="playerStore.toggleFavoritePath(entry.path)"
							/>
						</template>
					</ListRow>
				</div>
			</RecycleScroller>
		</div>

		<p v-if="currentPath" class="mt-3 min-w-0 break-words text-xs text-tx-muted">
			{{ t('favorites.current').replace('{0}', () => extractTrackName(currentPath)) }}
		</p>
	</section>
</template>
