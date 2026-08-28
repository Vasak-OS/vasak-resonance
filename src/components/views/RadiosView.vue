<script setup lang="ts">
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onBeforeUnmount, onMounted, type Ref, ref } from 'vue';
import LabeledField from '@/components/layout/LabeledField.vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';
import type { RadioStation } from '@/services/radio.service';
import {
	fetchRadioStations,
	getCachedStations,
	playRadioStation,
	setCachedStations,
} from '@/services/radio.service';

const { t } = useI18n();
const playIcon = useReactiveIcon('media-playback-start');
const searchIcon = useReactiveIcon('file-search');
const stations: Ref<RadioStation[]> = ref([]);
const loading = ref(false);
const error = ref('');
const selectedTag = ref('lofi');
const searchQuery = ref('');
const bufferingStationUuid = ref<string | null>(null);
const lastRequestedUrl = ref('');

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

async function loadStations() {
	loading.value = true;
	error.value = '';

	try {
		// Check cache first
		const cached = getCachedStations();
		if (cached && cached.length > 0) {
			stations.value = cached;
		}

		// Fetch fresh data
		const freshStations = await fetchRadioStations([selectedTag.value]);
		stations.value = freshStations;
		setCachedStations(freshStations);
	} catch (err) {
		const errorMsg = err instanceof Error ? err.message : String(err);
		error.value = t('radios.loadError').replace('{0}', () => errorMsg);
		console.error('Radio stations error:', err);

		// If we have cached stations, keep them available
		const cached = getCachedStations();
		if (cached && cached.length > 0) {
			stations.value = cached;
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
	<div class="flex flex-col h-full gap-4 overflow-hidden">
		<!-- Header with controls -->
		<div class="flex flex-col gap-2 px-4 pt-4">
			<h1 class="text-2xl font-bold">{{ t('radios.title') }}</h1>

			<!-- Tag selection -->
			<div v-once class="flex gap-2 flex-wrap">
				<button
					v-for="tag in availableTags"
					:key="tag"
					@click="selectedTag = tag; loadStations()"
					:class="[
						'px-3 py-1 rounded-full text-sm transition-colors',
						selectedTag === tag
							? 'bg-primary text-tx-on-primary'
							: 'bg-secondary text-tx-on-primary',
					]"
				>
					{{ tag }}
				</button>
			</div>

			<!-- Search -->
			<LabeledField :label="t('common.search')" class="flex-1">
				<div class="flex items-center gap-2 px-3 py-2 bg-ui-surface/80 rounded-corner">
					<img :src="searchIcon" :alt="t('common.search')" class="w-4 h-4" />
					<input
						v-model="searchQuery"
						type="text"
						:placeholder="t('radios.searchPlaceholder')"
						class="bg-transparent flex-1 text-sm"
					/>
				</div>
			</LabeledField>
		</div>

		<!-- Error message -->
		<div v-if="error" class="px-4 py-2 bg-status-error/15 text-status-error rounded mx-4 text-sm flex justify-between items-center">
			<span>{{ error }}</span>
			<button
				@click="loadStations()"
				:disabled="loading"
				class="ml-2 px-2 py-1 bg-status-error hover:bg-status-error/85 disabled:bg-status-error/50 rounded text-xs whitespace-nowrap"
			>
				{{ loading ? t('radios.retrying') : t('radios.retry') }}
			</button>
		</div>

		<!-- Stations list -->
		<div class="flex-1 overflow-y-auto px-4">
			<div v-if="loading" class="flex justify-center items-center h-full">
				<div class="text-tx-muted">{{ t('radios.loading') }}</div>
			</div>

			<div v-else-if="sortedStations.length === 0" class="flex justify-center items-center h-full">
				<div class="text-tx-muted">{{ t('radios.empty') }}</div>
			</div>

			<div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pb-4">
				<div
					v-for="station in sortedStations"
					:key="station.uuid"
					class="bg-ui-surface/80 rounded-corner p-3 hover:bg-ui-bg/80 transition-colors cursor-pointer flex gap-3"
					@click="handlePlayStation(station)"
				>
					<!-- Station icon/image -->
					<div class="flex-shrink-0">
						<img
							v-if="station.favicon"
							:src="station.favicon"
							:alt="station.name"
							class="w-12 h-12 rounded-corner"
							onerror="this.style.display='none'"
						/>
						<div v-else class="w-12 h-12 bg-primary rounded-corner flex items-center justify-center">
							<img :src="playIcon" :alt="t('radios.stationIconAlt')" class="w-6 h-6" />
						</div>
					</div>

					<!-- Station info -->
					<div class="flex-1 min-w-0">
						<h3 class="font-semibold text-sm truncate">{{ station.name }}</h3>
						<p class="text-xs text-tx-muted truncate">{{ station.country || t('radios.unknownCountry') }}</p>
						<div class="flex gap-1 mt-1">
							<span
								v-if="station.codec"
								class="text-xs bg-secondary px-2 py-0.5 rounded text-tx-on-primary"
							>
								{{ station.codec }}
							</span>
							<span
								v-if="station.bitrate"
								class="text-xs bg-secondary px-2 py-0.5 rounded text-tx-on-primary"
							>
								{{ station.bitrate }} kbps
							</span>
						</div>
						<p v-if="station.votes" class="text-xs text-tx-muted mt-1">👍 {{ station.votes }}</p>
					</div>

					<!-- Play button overlay -->
					<div class="flex-shrink-0 flex items-center">
						<div class="relative">
							<button
								class="p-2 bg-secondary rounded-full hover:bg-primary transition-colors"
								@click.stop="handlePlayStation(station)" :aria-label="t('common.play')">
								<img :src="playIcon" :alt="t('common.play')" class="w-5 h-5" />
							</button>
							<!-- buffering indicator -->
							<div v-if="bufferingStationUuid === station.uuid" class="absolute inset-0 flex items-center justify-center">
								<div class="w-6 h-6 border-2 border-t-transparent rounded-full animate-spin border-white"></div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
/* Smooth scrolling */
div {
	scroll-behavior: smooth;
}
</style>
