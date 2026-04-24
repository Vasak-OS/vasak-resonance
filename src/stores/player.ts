import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import {
	type DroppedPlaybackTrack,
	handleDroppedFile,
	type NowPlayingMetadata,
	type PlaybackProgressEvent,
	pausePlayback,
	playFile,
	resumePlayback,
	seekPlayback,
	setPlaybackVolume,
	stopPlayback as stopPlaybackCommand,
} from '@/services/player.service';

export const usePlayerStore = defineStore('player', () => {
	const currentTrack = ref<DroppedPlaybackTrack | null>(null);
	const currentPath = ref<string | null>(null);
	const queue = ref<string[]>([]);
	const history = ref<string[]>([]);
	const favoritePaths = ref<string[]>([]);
	const trackCacheByPath = ref<Record<string, DroppedPlaybackTrack>>({});
	const positionSeconds = ref(0);
	const durationSeconds = ref<number | null>(null);
	const isPlaying = ref(false);
	const isPaused = ref(false);
	const volume = ref(1);
	const busy = ref(false);
	const isAdvancingQueue = ref(false);
	const lastAutoAdvancedPath = ref<string | null>(null);
	const error = ref('');
	const isDragOver = ref(false);
	const globalBadgeMessage = ref('');
	let globalBadgeTimeout: number | null = null;
	let unlistenProgress: UnlistenFn | null = null;
	let unlistenMprisNext: UnlistenFn | null = null;
	let unlistenMprisPrevious: UnlistenFn | null = null;
	let unlistenMprisStop: UnlistenFn | null = null;

	const progressPercent = computed(() => {
		if (!durationSeconds.value || durationSeconds.value <= 0) {
			return 0;
		}
		return Math.min(100, (positionSeconds.value / durationSeconds.value) * 100);
	});

	const hasTrack = computed(() => Boolean(currentPath.value));
	const queuedCount = computed(() => queue.value.length);
	const trackCacheList = computed(() => Object.values(trackCacheByPath.value));
	const favoriteEntries = computed(() =>
		favoritePaths.value.map((path) => ({
			path,
			metadata: trackCacheByPath.value[path] ?? null,
		}))
	);
	const nextSuggestionPath = computed(() => {
		const current = currentPath.value;
		for (let index = history.value.length - 1; index >= 0; index -= 1) {
			const candidate = history.value[index];
			if (candidate && candidate !== current) {
				return candidate;
			}
		}
		return null;
	});
	const hasNextTrack = computed(() => queuedCount.value > 0 || Boolean(nextSuggestionPath.value));
	const nextActionLabel = computed(() =>
		queuedCount.value > 0 ? 'Next' : nextSuggestionPath.value ? 'Sugerido' : 'Next'
	);
	const isCurrentFavorite = computed(() => {
		if (!currentPath.value) {
			return false;
		}
		return favoritePaths.value.includes(currentPath.value);
	});

	const extractTrackName = (path: string): string => {
		const normalized = path.replace(/\\/g, '/');
		const parts = normalized.split('/');
		return parts[parts.length - 1] || path;
	};

	const syncFavoritesFromStorage = () => {
		if (typeof window === 'undefined') {
			return;
		}

		const raw = window.localStorage.getItem('resonance.favorites');
		if (!raw) {
			favoritePaths.value = [];
			return;
		}

		try {
			const parsed = JSON.parse(raw);
			favoritePaths.value = Array.isArray(parsed)
				? parsed.filter((value): value is string => typeof value === 'string' && value.length > 0)
				: [];
		} catch {
			favoritePaths.value = [];
		}
	};

	const persistFavorites = () => {
		if (typeof window === 'undefined') {
			return;
		}

		window.localStorage.setItem('resonance.favorites', JSON.stringify(favoritePaths.value));
	};

	const syncTrackCacheFromStorage = () => {
		if (typeof window === 'undefined') {
			return;
		}

		const raw = window.localStorage.getItem('resonance.track-cache');
		if (!raw) {
			trackCacheByPath.value = {};
			return;
		}

		try {
			const parsed = JSON.parse(raw) as Record<string, DroppedPlaybackTrack>;
			if (!parsed || typeof parsed !== 'object') {
				trackCacheByPath.value = {};
				return;
			}

			const sanitized: Record<string, DroppedPlaybackTrack> = {};
			for (const [path, track] of Object.entries(parsed)) {
				if (!path || !track || typeof track !== 'object') {
					continue;
				}
				sanitized[path] = {
					path,
					title:
						typeof track.title === 'string' && track.title ? track.title : extractTrackName(path),
					artist:
						typeof track.artist === 'string' && track.artist ? track.artist : 'Unknown Artist',
					album: typeof track.album === 'string' && track.album ? track.album : 'Unknown Album',
					duration_seconds: typeof track.duration_seconds === 'number' ? track.duration_seconds : 0,
					cover_data_url:
						typeof track.cover_data_url === 'string' || track.cover_data_url === null
							? track.cover_data_url
							: null,
				};
			}

			trackCacheByPath.value = sanitized;
		} catch {
			trackCacheByPath.value = {};
		}
	};

	const persistTrackCache = () => {
		if (typeof window === 'undefined') {
			return;
		}

		window.localStorage.setItem('resonance.track-cache', JSON.stringify(trackCacheByPath.value));
	};

	const cacheTrackMetadata = (track: DroppedPlaybackTrack) => {
		if (!track.path) {
			return;
		}

		trackCacheByPath.value = {
			...trackCacheByPath.value,
			[track.path]: track,
		};
		persistTrackCache();
	};

	const getTrackMetadata = (path: string) => {
		return trackCacheByPath.value[path] ?? null;
	};

	const ensureMetadataForPath = async (path: string) => {
		if (!path || trackCacheByPath.value[path]) {
			return trackCacheByPath.value[path] ?? null;
		}

		try {
			const track = await handleDroppedFile(path);
			cacheTrackMetadata(track);
			return track;
		} catch {
			return null;
		}
	};

	const ensureMetadataForPaths = async (paths: string[]) => {
		const uniquePaths = Array.from(
			new Set(paths.filter((path) => path && !trackCacheByPath.value[path]))
		);
		if (uniquePaths.length === 0) {
			return;
		}

		await Promise.allSettled(uniquePaths.map((path) => ensureMetadataForPath(path)));
	};

	const ensureMetadataForFavorites = async () => {
		await ensureMetadataForPaths(favoritePaths.value);
	};

	const showGlobalBadge = (message: string, durationMs = 2600) => {
		globalBadgeMessage.value = message;
		if (typeof window === 'undefined') {
			return;
		}

		if (globalBadgeTimeout !== null) {
			window.clearTimeout(globalBadgeTimeout);
		}

		globalBadgeTimeout = window.setTimeout(() => {
			globalBadgeMessage.value = '';
			globalBadgeTimeout = null;
		}, durationMs);
	};

	const shouldAutoAdvancePlayback = (payload: PlaybackProgressEvent): boolean => {
		if (isAdvancingQueue.value) {
			return false;
		}

		if (!payload.path || payload.is_playing || payload.is_paused) {
			return false;
		}

		if (payload.duration_seconds === null) {
			return false;
		}

		if (payload.position_seconds < payload.duration_seconds) {
			return false;
		}

		if (!hasNextTrack.value) {
			return false;
		}

		return lastAutoAdvancedPath.value !== payload.path;
	};

	const playNextInQueue = async () => {
		if (isAdvancingQueue.value) {
			return;
		}

		const [nextPath, ...rest] = queue.value;
		if (!nextPath) {
			return;
		}

		queue.value = rest;
		isAdvancingQueue.value = true;
		try {
			await playDropped(nextPath);
		} catch {
			if (queue.value.length > 0) {
				await playNextInQueue();
			}
		} finally {
			isAdvancingQueue.value = false;
		}
	};

	const playSuggestedTrack = async () => {
		const suggestedPath = nextSuggestionPath.value;
		if (!suggestedPath) {
			return false;
		}

		await playDropped(suggestedPath, false);
		return true;
	};

	const advancePlayback = async () => {
		if (queue.value.length > 0) {
			await playNextInQueue();
			return;
		}

		await playSuggestedTrack();
	};

	const applyProgress = (payload: PlaybackProgressEvent) => {
		currentPath.value = payload.path;
		positionSeconds.value = payload.position_seconds;
		durationSeconds.value = payload.duration_seconds;
		isPlaying.value = payload.is_playing;
		isPaused.value = payload.is_paused;
		volume.value = payload.volume;

		if (payload.now_playing) {
			const nowPlaying: NowPlayingMetadata = payload.now_playing;
			currentTrack.value = {
				path: nowPlaying.path,
				title: nowPlaying.title,
				artist: nowPlaying.artist,
				album: nowPlaying.album,
				duration_seconds: nowPlaying.duration_seconds,
				cover_data_url: nowPlaying.cover_data_url,
			};
			cacheTrackMetadata(currentTrack.value);
		} else if (!payload.path) {
			currentTrack.value = null;
		}

		if (payload.is_playing) {
			lastAutoAdvancedPath.value = null;
			return;
		}

		if (shouldAutoAdvancePlayback(payload)) {
			lastAutoAdvancedPath.value = payload.path;
			void advancePlayback();
		}
	};

	const initProgressListener = async () => {
		if (unlistenProgress) {
			return;
		}

		unlistenProgress = await listen<PlaybackProgressEvent>('audio-playback-progress', (event) => {
			applyProgress(event.payload);
		});
	};

	const initMprisNextListener = async () => {
		if (unlistenMprisNext) {
			return;
		}

		unlistenMprisNext = await listen('mpris-next-request', () => {
			void advancePlayback();
		});
	};

	const initMprisPreviousListener = async () => {
		if (unlistenMprisPrevious) {
			return;
		}

		unlistenMprisPrevious = await listen('mpris-previous-request', () => {
			void playPreviousTrack();
		});
	};

	const initMprisStopListener = async () => {
		if (unlistenMprisStop) {
			return;
		}

		unlistenMprisStop = await listen('mpris-stop-request', () => {
			void stopPlayback();
		});
	};

	const disposeProgressListener = () => {
		if (unlistenProgress) {
			unlistenProgress();
			unlistenProgress = null;
		}
	};

	const disposeMprisNextListener = () => {
		if (unlistenMprisNext) {
			unlistenMprisNext();
			unlistenMprisNext = null;
		}
	};

	const disposeMprisPreviousListener = () => {
		if (unlistenMprisPrevious) {
			unlistenMprisPrevious();
			unlistenMprisPrevious = null;
		}
	};

	const disposeMprisStopListener = () => {
		if (unlistenMprisStop) {
			unlistenMprisStop();
			unlistenMprisStop = null;
		}
	};

	const play = async (filePath: string) => {
		busy.value = true;
		error.value = '';
		try {
			await playFile(filePath);
			currentPath.value = filePath;
		} catch (playError: unknown) {
			error.value = `No se pudo reproducir el archivo: ${String(playError)}`;
		} finally {
			busy.value = false;
		}
	};

	const playDropped = async (filePath: string, recordHistory = true) => {
		console.log('[playDropped] Iniciando reproducción de:', filePath);
		busy.value = true;
		error.value = '';
		try {
			console.log('[playDropped] Llamando handleDroppedFile...');
			const track = await handleDroppedFile(filePath);
			console.log('[playDropped] Track obtenido:', track);
			if (recordHistory && currentPath.value && currentPath.value !== track.path) {
				history.value.push(currentPath.value);
			}
			currentTrack.value = track;
			cacheTrackMetadata(track);
			currentPath.value = track.path;
			durationSeconds.value = track.duration_seconds;
			console.log('[playDropped] Llamando playFile con:', track.path);
			await playFile(track.path);
			console.log('[playDropped] playFile completado exitosamente');
		} catch (dropError: unknown) {
			console.error('[playDropped] Error:', dropError);
			error.value = `No se pudo cargar el archivo arrastrado: ${String(dropError)}`;
		} finally {
			busy.value = false;
		}
	};

	const pause = async () => {
		try {
			error.value = '';
			await pausePlayback();
		} catch (pauseError: unknown) {
			error.value = `No se pudo pausar: ${String(pauseError)}`;
		}
	};

	const resume = async () => {
		try {
			error.value = '';
			await resumePlayback();
		} catch (resumeError: unknown) {
			error.value = `No se pudo reanudar: ${String(resumeError)}`;
		}
	};

	const togglePlayPause = async () => {
		if (!hasTrack.value) {
			return;
		}
		if (isPaused.value) {
			await resume();
		} else {
			await pause();
		}
	};

	const seekTo = async (seconds: number) => {
		try {
			error.value = '';
			await seekPlayback(seconds);
		} catch (seekError: unknown) {
			error.value = `No se pudo mover la reproducción: ${String(seekError)}`;
		}
	};

	const setVolume = async (nextVolume: number) => {
		const normalized = Math.min(2, Math.max(0, nextVolume));
		volume.value = normalized;
		try {
			error.value = '';
			await setPlaybackVolume(normalized);
		} catch (volumeError: unknown) {
			error.value = `No se pudo ajustar el volumen: ${String(volumeError)}`;
		}
	};

	const stopPlayback = async () => {
		try {
			error.value = '';
			await stopPlaybackCommand();
		} catch (stopError: unknown) {
			error.value = `No se pudo detener la reproducción: ${String(stopError)}`;
		}
	};

	const playPreviousTrack = async () => {
		if (hasTrack.value && positionSeconds.value > 5) {
			await seekTo(0);
			return;
		}

		const previousPath = history.value.pop();
		if (!previousPath) {
			await seekTo(0);
			return;
		}

		await playDropped(previousPath, false);
	};

	const setDragOver = (value: boolean) => {
		isDragOver.value = value;
	};

	const clearQueue = () => {
		queue.value = [];
	};

	const isFavoritePath = (path: string) => {
		return favoritePaths.value.includes(path);
	};

	const toggleFavoritePath = (path: string) => {
		if (!path) {
			return;
		}

		const metadata = getTrackMetadata(path);
		const trackLabel = metadata?.title || extractTrackName(path);

		if (isFavoritePath(path)) {
			favoritePaths.value = favoritePaths.value.filter((entry) => entry !== path);
			showGlobalBadge(`Quitado de favoritos: ${trackLabel}`);
		} else {
			favoritePaths.value = [...favoritePaths.value, path];
			showGlobalBadge(`Añadido a favoritos: ${trackLabel}`);
		}

		persistFavorites();
	};

	const toggleCurrentFavorite = () => {
		if (!currentPath.value) {
			return;
		}

		toggleFavoritePath(currentPath.value);
	};

	const removeQueueItem = (index: number) => {
		if (index < 0 || index >= queue.value.length) {
			return;
		}

		queue.value = queue.value.filter((_, itemIndex) => itemIndex !== index);
	};

	const moveQueueItem = (fromIndex: number, toIndex: number) => {
		if (
			fromIndex < 0 ||
			fromIndex >= queue.value.length ||
			toIndex < 0 ||
			toIndex >= queue.value.length ||
			fromIndex === toIndex
		) {
			return;
		}

		const nextQueue = [...queue.value];
		const [movedItem] = nextQueue.splice(fromIndex, 1);
		nextQueue.splice(toIndex, 0, movedItem);
		queue.value = nextQueue;
	};

	const handleDroppedPaths = async (paths: string[]) => {
		console.log('[handleDroppedPaths] Paths recibidos:', paths);
		const normalized = Array.from(
			new Set(paths.map((path) => path.trim()).filter((path) => path.length > 0))
		);
		console.log('[handleDroppedPaths] Paths normalizados:', normalized);
		const [firstPath, ...rest] = normalized;
		if (!firstPath) {
			console.warn('[handleDroppedPaths] No hay primera ruta para reproducir');
			return;
		}

		queue.value = rest;
		void ensureMetadataForPaths(normalized);
		console.log('[handleDroppedPaths] Llamando playDropped con:', firstPath);
		await playDropped(firstPath);
	};

	const enqueuePaths = (paths: string[]) => {
		const normalized = Array.from(
			new Set(paths.map((path) => path.trim()).filter((path) => path.length > 0))
		);

		if (normalized.length === 0) {
			return;
		}

		const existing = new Set(queue.value);
		const nextItems = normalized.filter((path) => !existing.has(path));
		if (nextItems.length === 0) {
			return;
		}

		queue.value = [...queue.value, ...nextItems];
		void ensureMetadataForPaths(nextItems);
	};

	const playAlbum = async (paths: string[]) => {
		const normalized = Array.from(
			new Set(paths.map((path) => path.trim()).filter((path) => path.length > 0))
		);
		const [firstPath, ...restPaths] = normalized;
		if (!firstPath) {
			return;
		}

		const existingAfterAlbum = queue.value.filter((path) => !restPaths.includes(path));
		queue.value = [...restPaths, ...existingAfterAlbum];
		void ensureMetadataForPaths(normalized);
		await playDropped(firstPath);
	};

	const advanceQueue = async () => {
		await advancePlayback();
	};

	syncTrackCacheFromStorage();
	syncFavoritesFromStorage();
	void ensureMetadataForFavorites();

	return {
		advanceQueue,
		busy,
		clearQueue,
		currentPath,
		currentTrack,
		durationSeconds,
		error,
		handleDroppedPaths,
		hasTrack,
		hasNextTrack,
		initProgressListener,
		initMprisNextListener,
		initMprisPreviousListener,
		initMprisStopListener,
		history,
		favoritePaths,
		favoriteEntries,
		globalBadgeMessage,
		getTrackMetadata,
		enqueuePaths,
		showGlobalBadge,
		playAlbum,
		isCurrentFavorite,
		isFavoritePath,
		queue,
		queuedCount,
		nextActionLabel,
		nextSuggestionPath,
		trackCacheList,
		ensureMetadataForFavorites,
		ensureMetadataForPath,
		moveQueueItem,
		removeQueueItem,
		isDragOver,
		isPaused,
		isPlaying,
		pause,
		play,
		playDropped,
		playPreviousTrack,
		positionSeconds,
		progressPercent,
		resume,
		seekTo,
		setDragOver,
		setVolume,
		toggleCurrentFavorite,
		toggleFavoritePath,
		togglePlayPause,
		disposeProgressListener,
		disposeMprisNextListener,
		disposeMprisPreviousListener,
		disposeMprisStopListener,
		stopPlayback,
		volume,
	};
});
