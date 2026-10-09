<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Badge,
	EmptyState,
	FormGroup,
	LoadingState,
	SearchField,
	SegmentedControl,
	type SegmentedOption,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, type Ref, reactive, ref, watch } from 'vue';
import { RecycleScroller } from 'vue-virtual-scroller';
import { useVisibleColumns } from '@/composables/useVisibleColumns';
import type { RadioStation } from '@/services/radio.service';
import {
	fetchRadioStations,
	getCachedStations,
	getStaleCachedStations,
	playRadioStation,
	setCachedStations,
} from '@/services/radio.service';
import { inRows } from '@/tools/lists';

const { t } = useI18n();
const stations: Ref<RadioStation[]> = ref([]);
const loading = ref(false);
const error = ref('');
const selectedTag = ref('lofi');
const searchQuery = ref('');
const bufferingStationUuid = ref<string | null>(null);
const lastRequestedUrl = ref('');

/**
 * Las emisoras cuyo icono no cargó.
 *
 * Un `Set` reactivo y no una marca por emisora: la lista se reemplaza entera en
 * cada búsqueda, y guardar el estado adentro de cada elemento lo perdería.
 */
const brokenFavicons = reactive(new Set<string>());

// Y se vacía con cada lista nueva. Un UUID marcado se quedaba marcado hasta
// que la ventana se cerrara: si la emisora arreglaba su icono y la recarga lo
// traía bien, seguía dibujándose el de la aplicación. Lo marcó la revisión.
// Se vacía al reemplazar la lista y no al recibir cada icono porque el `@error`
// es lo único que avisa: no hay forma de preguntar si hoy carga sin intentarlo.
watch(stations, () => {
	brokenFavicons.clear();
});

const availableTags = [
	'lofi',
	'synthwave',
	'jazz',
	'ambient',
	'chillhop',
	'classical',
	'electronic',
	'indie',
	'metal',
	'pop',
	'rock',
	'hiphop',
];

const filteredStations = computed(() => {
	if (!searchQuery.value) {
		return stations.value;
	}

	const query = searchQuery.value.toLowerCase();
	return stations.value.filter(
		(station) =>
			station.name.toLowerCase().includes(query) ||
			station.country?.toLowerCase().includes(query) ||
			station.tags?.toLowerCase().includes(query)
	);
});

const sortedStations = computed(() => {
	return [...filteredStations.value].sort((a, b) => {
		// Sort by votes (higher first), then by name
		const aVotes = a.votes || 0;
		const bVotes = b.votes || 0;
		if (aVotes !== bVotes) {
			return bVotes - aVotes;
		}
		return a.name.localeCompare(b.name);
	});
});

/**
 * Las columnas que dibujaba `grid-cols-1 md:grid-cols-2 lg:grid-cols-3`, ahora
 * dichas a mano porque el scroller coloca las filas él y no puede leerlas del
 * CSS. Los cortes son los de Tailwind.
 */
const columns = useVisibleColumns([
	{ from: 768, columns: 2 },
	{ from: 1024, columns: 3 },
]);

const stationRows = computed(() =>
	inRows(sortedStations.value, columns.value, (station) => station.uuid)
);

/**
 * Lo que mide una fila de la cuadrícula, contando el hueco de abajo.
 *
 * El scroller coloca a partir de este número, así que tiene que coincidir con
 * lo que el CSS dibuja: 116 de alto de tarjeta más los 12 del `pb-3`. Si se
 * separan, las filas se pisan o dejan huecos y no falla nada — se ve torcido.
 */
const ROW_HEIGHT = 128;

/** Las etiquetas como filtro de una sola opción: cuál está puesta se ve y se anuncia. */
const tagOptions: SegmentedOption<string>[] = availableTags.map((tag) => ({
	label: tag,
	value: tag,
}));

const selectTag = (tag: string) => {
	selectedTag.value = tag;
	void loadStations();
};

async function loadStations() {
	loading.value = true;
	error.value = '';

	const tags = [selectedTag.value];

	try {
		// Lo guardado hace menos de una hora alcanza: se muestra y no se
		// pregunta. Antes se preguntaba igual siempre, así que abrir la vista
		// era una consulta al directorio aunque acabara de hacerse.
		const fresh = getCachedStations(tags);
		if (fresh) {
			stations.value = fresh;
			return;
		}

		// Lo viejo se muestra mientras llega lo nuevo, para no dejar la lista
		// en blanco. Y si no hay nada guardado para esta etiqueta, la lista se
		// vacía: dejar la de la etiqueta anterior la haría pasar por ésta —y si
		// además falla la consulta, se queda así—.
		stations.value = getStaleCachedStations(tags) ?? [];

		const freshStations = await fetchRadioStations(tags);
		stations.value = freshStations;
		setCachedStations(tags, freshStations);
	} catch (err) {
		const errorMsg = err instanceof Error ? err.message : String(err);
		error.value = t('radios.loadError').replace('{0}', () => errorMsg);
		console.error('Radio stations error:', err);

		// Sin el directorio, lo que haya guardado **de esta etiqueta** aunque
		// esté viejo: es justamente cuando más falta hace. Si no hay, la lista
		// queda vacía con su error, que es lo honesto.
		const cached = getStaleCachedStations(tags);
		stations.value = cached ?? [];
		if (cached) {
			error.value = t('radios.usingCache').replace('{0}', () => errorMsg);
		}
	} finally {
		loading.value = false;
	}
}

async function handlePlayStation(station: RadioStation) {
	try {
		bufferingStationUuid.value = station.uuid;
		lastRequestedUrl.value = station.url || '';
		await playRadioStation(station);
		// keep buffering indicator until playback event arrives
	} catch (err) {
		bufferingStationUuid.value = null;
		error.value = t('radios.playError').replace('{0}', () => String(err));
		console.error(err);
	}
}

let unlistenPlayback: (() => void) | null = null;

onMounted(async () => {
	// Load initial stations
	await loadStations();

	// Listen to backend playback snapshots to clear buffering indicator
	unlistenPlayback = await listen('audio-playback-progress', (event) => {
		const payload = (event as any).payload;
		if (!payload) return;
		// `path` and not `now_playing.path`: the metadata only rides along on the
		// tick where the track changes, and a stream is usually still buffering
		// at that point — reading it from there left the spinner up forever.
		// The backend reports the station URL here now.
		if (payload.path && lastRequestedUrl.value && payload.path === lastRequestedUrl.value) {
			if (payload.is_playing) {
				bufferingStationUuid.value = null;
			}
		}
	});
});

onBeforeUnmount(() => {
	if (unlistenPlayback) {
		unlistenPlayback();
	}
});
</script>

<template>
	<div class="flex h-full flex-col gap-4 overflow-hidden">
		<!-- Header with controls -->
		<div class="flex flex-col gap-2 px-4 pt-4">
			<h1 class="text-2xl font-bold">{{ t('radios.title') }}</h1>

			<!-- Tag selection: una sola puesta a la vez, y se ve cuál. -->
			<SegmentedControl
				:model-value="selectedTag"
				:options="tagOptions"
				:label="t('radios.tagsLabel')"
				variant="chips"
				@change="selectTag"
			/>

			<!-- Search -->
			<FormGroup class="flex-1" :label="t('common.search')" variant="eyebrow">
				<SearchField v-model="searchQuery" :label="t('common.search')" :placeholder="t('radios.searchPlaceholder')" />
			</FormGroup>
		</div>

		<!-- Error message, con el reintento al lado: en franja, como antes, que
		     pone el botón en la misma línea cuando hay lugar. -->
		<AlertMessage v-if="error" class="mx-4 rounded-corner-m border" variant="banner" tone="error" icon="auto">
			{{ error }}
			<template #actions>
				<ActionButton
					:label="loading ? t('radios.retrying') : t('radios.retry')"
					:loading="loading"
					variant="secondary"
					size="sm"
					@click="loadStations()"
				/>
			</template>
		</AlertMessage>

		<!-- Stations list -->
		<div class="min-h-0 flex-1 overflow-hidden px-4">
			<div v-if="loading" class="flex h-full items-center justify-center">
				<LoadingState :label="t('radios.loading')" />
			</div>

			<div v-else-if="sortedStations.length === 0" class="flex h-full items-center justify-center">
				<EmptyState :title="t('radios.empty')" />
			</div>

			<!--
				Se virtualiza **por filas**: una fila de la cuadrícula es un
				elemento del scroller y adentro va el `grid` de siempre. Es el
				mismo camino que `AlbumsView`, y evita tener que decirle al
				scroller el ancho de cada columna en píxeles.

				La tarjeta pasa a tener alto fijo. Antes variaba —la línea de
				votos sólo aparece cuando la emisora los trae—, y una cuadrícula
				de altos distintos no se puede colocar sin medir. De paso se ve
				mejor pareja.
			-->
			<RecycleScroller
				v-else
				:items="stationRows"
				key-field="key"
				:item-size="ROW_HEIGHT"
				class="h-full"
				v-slot="{ item: row }"
			>
				<div
					class="grid gap-3 pb-3"
					:style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }"
				>
				<div
					v-for="station in row.items"
					:key="station.uuid"
					class="flex h-[116px] cursor-pointer gap-3 overflow-hidden rounded-corner-l border border-ui-line bg-ui-surface/70 p-3 transition-colors duration-200 ease-ui hover:bg-ui-hover"
					@click="handlePlayStation(station)"
				>
					<!-- Station icon/image -->
					<div class="shrink-0">
						<!-- El icono de la emisora, y el de la aplicación cuando no hay o
						     no carga. Acá había un `onerror="this.style.display='none'"`,
						     que es un manejador **en línea**: la política de contenido de
						     esta ventana no permite `script-src` inline, así que el
						     navegador nunca lo ejecutaba y un favicon roto dejaba el
						     dibujo de imagen rota. Con `@error` lo maneja Vue, y en vez de
						     esconder el `<img>` se muestra el icono de abajo, que es lo
						     que se ve cuando la emisora no trae ninguno. -->
						<img
							v-if="station.favicon && !brokenFavicons.has(station.uuid)"
							:src="station.favicon"
							:alt="station.name"
							class="size-12 rounded-corner-m"
							@error="brokenFavicons.add(station.uuid)"
						/>
						<div v-else class="flex size-12 items-center justify-center rounded-corner-m bg-ui-selected-accent">
							<ThemeIcon
								name="media-playback-start"
								type="symbol"
								:size="24"
								:alt="t('radios.stationIconAlt')"
							/>
						</div>
					</div>

					<!-- Station info -->
					<div class="min-w-0 flex-1">
						<h3 class="truncate text-sm font-semibold">{{ station.name }}</h3>
						<p class="truncate text-xs text-tx-muted">{{ station.country || t('radios.unknownCountry') }}</p>
						<div class="mt-1 flex min-w-0 flex-wrap gap-1">
							<Badge v-if="station.codec">{{ station.codec }}</Badge>
							<Badge v-if="station.bitrate">{{ station.bitrate }} kbps</Badge>
						</div>
						<p v-if="station.votes" class="mt-1 flex items-center gap-1 text-xs text-tx-muted">
							<ThemeIcon name="emblem-favorite-symbolic" type="symbol" :size="12" :alt="t('radios.votes')" />
							{{ station.votes }}
						</p>
					</div>

					<!-- Play button, con la rueda en su lugar mientras carga -->
					<div class="flex shrink-0 items-center">
						<ActionButton
							label=""
							:icon-alt="t('common.play')"
							:title="t('common.play')"
							icon="media-playback-start"
							:loading="bufferingStationUuid === station.uuid"
							stop-propagation
							@click="handlePlayStation(station)"
						/>
					</div>
				</div>
				</div>
			</RecycleScroller>
		</div>
	</div>
</template>
