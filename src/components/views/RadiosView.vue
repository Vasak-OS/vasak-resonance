<script setup lang="ts">
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, type Ref, ref } from 'vue';
import LabeledField from '@/components/layout/LabeledField.vue';
import type { RadioStation } from '@/services/radio.service';
import {
	fetchRadioStations,
	getCachedStations,
	playRadioStation,
	setCachedStations,
} from '@/services/radio.service';
import { usePlayerStore } from '@/stores/player';

const playerStore = usePlayerStore();
const playIcon = ref('');
const searchIcon = ref('');
const stations: Ref<RadioStation[]> = ref([]);
const loading = ref(false);
const error = ref('');
const selectedTag = ref('lofi');
const searchQuery = ref('');

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
	return filteredStations.value.sort((a, b) => {
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
		error.value = `Failed to load stations: ${errorMsg}`;
		console.error('Radio stations error:', err);

		// If we have cached stations, keep them available
		const cached = getCachedStations();
		if (cached && cached.length > 0) {
			stations.value = cached;
			error.value = `Using cached stations (last loaded earlier). Error: ${errorMsg}`;
		}
	} finally {
		loading.value = false;
	}
}

async function handlePlayStation(station: RadioStation) {
	try {
		await playRadioStation(station);
	} catch (err) {
		error.value = `Error playing station: ${err}`;
		console.error(err);
	}
}

onMounted(async () => {
	const playSymbol = await getSymbolSource('media-playback-start');
	const searchSymbol = await getSymbolSource('file-search');
	playIcon.value = playSymbol;
	searchIcon.value = searchSymbol;

	// Load initial stations
	await loadStations();
});
</script>

<template>
	<div class="flex flex-col h-full gap-4 overflow-hidden">
		<!-- Header with controls -->
		<div class="flex flex-col gap-2 px-4 pt-4">
			<h1 class="text-2xl font-bold">Radio Stations</h1>

			<!-- Tag selection -->
			<div class="flex gap-2 flex-wrap">
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
			<LabeledField label="Search" class="flex-1">
				<div class="flex items-center gap-2 px-3 py-2 bg-ui-surface/80 rounded-corner">
					<img :src="searchIcon" alt="Search" class="w-4 h-4" />
					<input
						v-model="searchQuery"
						type="text"
						placeholder="Search stations..."
						class="bg-transparent flex-1 outline-none text-sm"
					/>
				</div>
			</LabeledField>
		</div>

		<!-- Error message -->
		<div v-if="error" class="px-4 py-2 bg-red-900 text-red-200 rounded mx-4 text-sm flex justify-between items-center">
			<span>{{ error }}</span>
			<button
				@click="loadStations()"
				:disabled="loading"
				class="ml-2 px-2 py-1 bg-red-700 hover:bg-red-600 disabled:bg-red-800 rounded text-xs whitespace-nowrap"
			>
				{{ loading ? 'Retrying...' : 'Retry' }}
			</button>
		</div>

		<!-- Stations list -->
		<div class="flex-1 overflow-y-auto px-4">
			<div v-if="loading" class="flex justify-center items-center h-full">
				<div class="text-tx-muted">Loading stations...</div>
			</div>

			<div v-else-if="sortedStations.length === 0" class="flex justify-center items-center h-full">
				<div class="text-tx-muted">No stations found</div>
			</div>

			<div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pb-4">
				<div
					v-for="station in sortedStations"
					:key="station.uuid"
					class="bg-ui-surface/80 rounded-lg p-3 hover:bg-ui-bg/80 transition-colors cursor-pointer flex gap-3"
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
							<img :src="playIcon" alt="Radio" class="w-6 h-6" />
						</div>
					</div>

					<!-- Station info -->
					<div class="flex-1 min-w-0">
						<h3 class="font-semibold text-sm truncate">{{ station.name }}</h3>
						<p class="text-xs text-gray-400 truncate">{{ station.country || 'Unknown' }}</p>
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
						<button
							class="p-2 bg-secondary rounded-full hover:bg-primary transition-colors"
							@click.stop="handlePlayStation(station)"
						>
							<img :src="playIcon" alt="Play" class="w-5 h-5" />
						</button>
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
