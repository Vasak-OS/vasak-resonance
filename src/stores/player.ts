import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { defineStore } from 'pinia';
import { computed, ref, shallowRef, watch } from 'vue';
import { devLog } from '@/composables/useDevLog';
import {
	type DroppedPlaybackTrack,
	getPlaybackSnapshot,
	handleDroppedFile,
	listLibraryTracks,
	type NowPlayingMetadata,
	type PlaybackProgressEvent,
	pausePlayback,
	playFile,
	resumePlayback,
	saveLibraryTrack,
	seekPlayback,
	setPlaybackVolume,
	stopPlayback as stopPlaybackCommand,
} from '@/services/player.service';
import {
	createQueueEntries,
	findQueueEntry,
	moveQueueEntry,
	type QueueEntry,
	queuePaths,
	removeQueueEntry,
} from '@/stores/playerQueue';

export const usePlayerStore = defineStore('player', () => {
	const { t } = useI18n();
	const currentTrack = ref<DroppedPlaybackTrack | null>(null);
	const currentPath = ref<string | null>(null);
	/**
	 * La cola de verdad. Cada entrada tiene identidad propia para que el menú
	 * del clic derecho siga apuntando a la canción que se señaló aunque la cola
	 * avance mientras el menú está abierto.
	 */
	const queueEntries = ref<QueueEntry[]>([]);
	/** Las rutas de la cola, para todo lo que sólo necesita saber qué suena. */
	const queue = computed(() => queuePaths(queueEntries.value));
	const history = ref<string[]>([]);
	const favoritePaths = ref<string[]>([]);
	const trackCacheByPath = shallowRef<Record<string, DroppedPlaybackTrack>>({});
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
	const isScanning = ref(false);
	let globalBadgeTimeout: number | null = null;
	let unlistenProgress: UnlistenFn | null = null;
	let unlistenTrackFinished: UnlistenFn | null = null;
	let unlistenMprisNext: UnlistenFn | null = null;
	let unlistenMprisPrevious: UnlistenFn | null = null;
	let unlistenMprisStop: UnlistenFn | null = null;
	let beforeUnloadHandler: (() => void) | null = null;
	const playbackStorage = ref<any | null>(null);

	const initPlaybackStorage = async () => {
		try {
			if (!playbackStorage.value) {
				const filePath = 'resonance-playback.json';
				try {
					const plugin = await import('@tauri-apps/plugin-store');
					playbackStorage.value = await new plugin.LazyStore(filePath);
					await playbackStorage.value.save();
				} catch (err) {
					console.warn(
						'[initPlaybackStorage] plugin-store not available, fallback to localStorage',
						err
					);
				}
			}
		} catch (err) {
			console.error('[initPlaybackStorage] failed:', err);
		}
	};

	const progressPercent = computed(() => {
		if (!durationSeconds.value || durationSeconds.value <= 0) {
			return 0;
		}
		return Math.min(100, (positionSeconds.value / durationSeconds.value) * 100);
	});

	const hasTrack = computed(() => Boolean(currentPath.value));
	// A station is identified by its URL, which is now what the backend reports
	// as the current path — it used to report nothing at all for radio, which is
	// why the transport controls did not work while one played.
	const isStream = computed(() => {
		const path = currentPath.value ?? currentTrack.value?.path ?? '';
		return path.startsWith('http://') || path.startsWith('https://');
	});
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
		queuedCount.value === 0 && nextSuggestionPath.value
			? t('transport.suggested')
			: t('transport.next')
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

			// «Unknown Artist» y «Unknown Album» son los centinelas que escribe el
			// backend, y `lyrics.rs` los reconoce para no salir a buscar la letra
			// de un artista que no existe: acá son dato, no texto de interfaz. La
			// traducción ocurre al mostrarlos, en `useMetadataLabels`.
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
					dominant_color:
						typeof track.dominant_color === 'string' || track.dominant_color === null
							? track.dominant_color
							: null,
				};
			}

			trackCacheByPath.value = sanitized;
		} catch {
			trackCacheByPath.value = {};
		}
	};

	/**
	 * How many tracks keep their metadata between runs.
	 *
	 * Each entry holds a base64 cover, so a handful of them is already
	 * megabytes. localStorage grants about 5 MB per origin, and an unbounded
	 * cache eventually fills it — at which point `setItem` throws and, before
	 * this was handled, took the playback update with it.
	 */
	const TRACK_CACHE_LIMIT = 40;

	const persistTrackCache = () => {
		if (typeof window === 'undefined') {
			return;
		}

		// Oldest entries first, so dropping from the front discards what was
		// added longest ago.
		let entries = Object.entries(trackCacheByPath.value);
		if (entries.length > TRACK_CACHE_LIMIT) {
			entries = entries.slice(-TRACK_CACHE_LIMIT);
			trackCacheByPath.value = Object.fromEntries(entries);
		}

		while (entries.length > 0) {
			try {
				window.localStorage.setItem(
					'resonance.track-cache',
					JSON.stringify(Object.fromEntries(entries))
				);
				return;
			} catch {
				// Out of quota: halve the cache and try again rather than losing
				// every cover, which would make the player flash blank artwork on
				// each track change.
				entries = entries.slice(Math.ceil(entries.length / 2));
				trackCacheByPath.value = Object.fromEntries(entries);
			}
		}

		try {
			window.localStorage.removeItem('resonance.track-cache');
		} catch {
			// Nothing further to do; the cache is a convenience.
		}
	};

	const persistPlaybackState = async () => {
		try {
			await initPlaybackStorage();
			if (playbackStorage.value) {
				const payload = {
					currentPath: currentPath.value,
					queue: queue.value,
					positionSeconds: positionSeconds.value,
					isPlaying: isPlaying.value,
					volume: volume.value,
				};
				await playbackStorage.value.set('playback', payload);
				await playbackStorage.value.save();
				return;
			}

			// Fallback to localStorage if plugin-store not available
			if (typeof window !== 'undefined') {
				const payload = {
					currentPath: currentPath.value,
					queue: queue.value,
					positionSeconds: positionSeconds.value,
					isPlaying: isPlaying.value,
					volume: volume.value,
				};
				try {
					window.localStorage.setItem('resonance.playback-state', JSON.stringify(payload));
				} catch {
					// ignore
				}
			}
		} catch (err) {
			console.error('[persistPlaybackState] failed:', err);
		}
	};

	const syncPlaybackStateFromStorage = async () => {
		try {
			await initPlaybackStorage();
			let parsed: any = null;
			if (playbackStorage.value) {
				parsed = await playbackStorage.value.get('playback');
			} else if (typeof window !== 'undefined') {
				const raw = window.localStorage.getItem('resonance.playback-state');
				if (raw) {
					parsed = JSON.parse(raw);
				}
			}

			if (!parsed) return;

			queueEntries.value = createQueueEntries(
				Array.isArray(parsed.queue)
					? (parsed.queue.filter(
							(p: unknown) => typeof p === 'string' && (p as string).length > 0
						) as string[])
					: []
			);
			void ensureMetadataForPaths(queue.value);

			if (typeof parsed.volume === 'number') {
				volume.value = parsed.volume;
				void setPlaybackVolume(parsed.volume);
			}

			if (parsed.currentPath) {
				await ensureMetadataForPath(parsed.currentPath);
				const restorePos =
					typeof parsed.positionSeconds === 'number' && parsed.positionSeconds > 0
						? parsed.positionSeconds
						: undefined;
				await playDropped(parsed.currentPath, true, restorePos);
				if (!parsed.isPlaying) {
					await pausePlayback();
				}
			}
		} catch (err) {
			console.error('[syncPlaybackStateFromStorage] failed:', err);
		}
	};

	const cacheTrackMetadata = (track: DroppedPlaybackTrack) => {
		if (!track.path) {
			return;
		}

		// Skip identical rewrites. Playback ticks arrive twice a second and used
		// to land here every time, so the whole cache — every cover ever seen,
		// base64-encoded — was re-serialised and written to localStorage 120
		// times a minute, on the main thread, for as long as music played.
		const cached = trackCacheByPath.value[track.path];
		if (cached && sameTrackMetadata(cached, track)) {
			return;
		}

		trackCacheByPath.value = {
			...trackCacheByPath.value,
			[track.path]: track,
		};
		persistTrackCache();
	};

	const sameTrackMetadata = (a: DroppedPlaybackTrack, b: DroppedPlaybackTrack): boolean =>
		a.title === b.title &&
		a.artist === b.artist &&
		a.album === b.album &&
		a.duration_seconds === b.duration_seconds &&
		a.cover_data_url === b.cover_data_url &&
		a.dominant_color === b.dominant_color;

	const setCurrentTrackVisuals = (coverDataUrl: string | null, dominantColor: string | null) => {
		if (!currentTrack.value?.path) {
			return;
		}

		currentTrack.value = {
			...currentTrack.value,
			cover_data_url: coverDataUrl,
			dominant_color: dominantColor,
		};

		cacheTrackMetadata(currentTrack.value);
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

	const playNextInQueue = async () => {
		if (isAdvancingQueue.value) {
			return;
		}

		// Se saca la primera entrada y las demás siguen siendo las mismas, con su
		// identificador intacto: si hay un menú abierto sobre alguna, sigue
		// hablando de esa canción y no de la que le quedó el lugar.
		const [nextEntry, ...rest] = queueEntries.value;
		if (!nextEntry) {
			return;
		}

		const nextPath = nextEntry.path;
		queueEntries.value = rest;
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

	const tryAutoAdvance = (payload: PlaybackProgressEvent) => {
		if (isAdvancingQueue.value) return;
		if (!payload.path || !hasNextTrack.value) return;
		if (lastAutoAdvancedPath.value === payload.path) return;

		const effectiveDuration = payload.duration_seconds ?? durationSeconds.value;
		if (!effectiveDuration || effectiveDuration <= 0) return;
		if (payload.position_seconds < effectiveDuration) return;

		lastAutoAdvancedPath.value = payload.path;
		void advancePlayback();
	};

	/**
	 * The backend saw the track play out.
	 *
	 * This is the authoritative end-of-track signal. Inferring it from the
	 * position reaching the duration only works when the file's tags declare
	 * one; without it, the track ended and the player just sat there — and at
	 * the end of the queue nothing ever cleared the playing state.
	 */
	const handleTrackFinished = (finishedPath: string | null) => {
		if (isAdvancingQueue.value) return;
		if (finishedPath && lastAutoAdvancedPath.value === finishedPath) return;
		lastAutoAdvancedPath.value = finishedPath;

		if (hasNextTrack.value) {
			void advancePlayback();
			return;
		}

		void stopPlayback();
	};

	const applyProgress = (payload: PlaybackProgressEvent) => {
		const prevPath = currentPath.value;
		currentPath.value = payload.path;
		const dur = payload.duration_seconds ?? durationSeconds.value;
		positionSeconds.value = dur
			? Math.min(payload.position_seconds, dur)
			: payload.position_seconds;
		if (prevPath !== payload.path) {
			durationSeconds.value = payload.duration_seconds;
		} else if (payload.duration_seconds !== null) {
			durationSeconds.value = payload.duration_seconds;
		}
		isPlaying.value = payload.is_playing;
		isPaused.value = payload.is_paused;
		volume.value = payload.volume;

		if (payload.now_playing) {
			const nowPlaying: NowPlayingMetadata = payload.now_playing;
			const cachedTrack = trackCacheByPath.value[nowPlaying.path] ?? null;
			currentTrack.value = {
				path: nowPlaying.path,
				title: nowPlaying.title,
				artist: nowPlaying.artist,
				album: nowPlaying.album,
				duration_seconds: nowPlaying.duration_seconds,
				// Preserve cached/embedded visuals when playback events don't include them.
				cover_data_url: nowPlaying.cover_data_url ?? cachedTrack?.cover_data_url ?? null,
				dominant_color: nowPlaying.dominant_color ?? cachedTrack?.dominant_color ?? null,
			};
			cacheTrackMetadata(currentTrack.value);
		} else if (!payload.path) {
			currentTrack.value = null;
		}

		if (payload.is_playing) {
			lastAutoAdvancedPath.value = null;
		}

		tryAutoAdvance(payload);
	};

	const initProgressListener = async () => {
		if (unlistenProgress) {
			return;
		}

		unlistenProgress = await listen<PlaybackProgressEvent>('audio-playback-progress', (event) => {
			applyProgress(event.payload);
		});

		unlistenTrackFinished = await listen<string | null>('audio-track-finished', (event) => {
			handleTrackFinished(event.payload);
		});
	};

	const syncPlaybackSnapshot = async () => {
		try {
			const snapshot = await getPlaybackSnapshot();
			applyProgress(snapshot);
		} catch {
			// Ignorado: el listener periódico actualizará el estado si falla la lectura inicial.
		}
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
		if (unlistenTrackFinished) {
			unlistenTrackFinished();
			unlistenTrackFinished = null;
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
			error.value = t('player.playError').replace('{0}', String(playError));
		} finally {
			busy.value = false;
		}
	};

	const playDropped = async (filePath: string, recordHistory = true, seekTo_seconds?: number) => {
		devLog('[playDropped] Iniciando reproducción de:', filePath);
		busy.value = true;
		error.value = '';
		try {
			const cached = trackCacheByPath.value[filePath];
			const track = cached ?? (await handleDroppedFile(filePath));
			devLog('[playDropped] Track obtenido:', cached ? '(desde cache)' : track);

			if (!cached) {
				await saveLibraryTrack(track);
				console.log('[playDropped] Track sincronizado en SQLite');
			}

			if (recordHistory && currentPath.value && currentPath.value !== track.path) {
				history.value.push(currentPath.value);
			}
			currentTrack.value = track;
			cacheTrackMetadata(track);
			currentPath.value = track.path;
			durationSeconds.value = track.duration_seconds;
			devLog('[playDropped] Llamando playFile con:', track.path, seekTo_seconds);
			await playFile(track.path, seekTo_seconds);
			devLog('[playDropped] playFile completado exitosamente');
		} catch (dropError: unknown) {
			console.error('[playDropped] Error:', dropError);
			error.value = t('player.dropError').replace('{0}', String(dropError));
		} finally {
			busy.value = false;
		}
	};

	const pause = async () => {
		try {
			error.value = '';
			await pausePlayback();
		} catch (pauseError: unknown) {
			error.value = t('player.pauseError').replace('{0}', String(pauseError));
		}
	};

	const resume = async () => {
		try {
			error.value = '';
			await resumePlayback();
		} catch (resumeError: unknown) {
			error.value = t('player.resumeError').replace('{0}', String(resumeError));
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
			error.value = t('player.seekError').replace('{0}', String(seekError));
		}
	};

	const setVolume = async (nextVolume: number) => {
		const normalized = Math.min(2, Math.max(0, nextVolume));
		volume.value = normalized;
		try {
			error.value = '';
			await setPlaybackVolume(normalized);
		} catch (volumeError: unknown) {
			error.value = t('player.volumeError').replace('{0}', String(volumeError));
		}
	};

	const stopPlayback = async () => {
		try {
			error.value = '';
			await stopPlaybackCommand();
		} catch (stopError: unknown) {
			error.value = t('player.stopError').replace('{0}', String(stopError));
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
		queueEntries.value = [];
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
			showGlobalBadge(t('favorites.removedBadge').replace('{0}', trackLabel));
		} else {
			favoritePaths.value = [...favoritePaths.value, path];
			showGlobalBadge(t('favorites.addedBadge').replace('{0}', trackLabel));
		}

		persistFavorites();
	};

	const toggleCurrentFavorite = () => {
		if (!currentPath.value) {
			return;
		}

		toggleFavoritePath(currentPath.value);
	};

	/**
	 * Quitar de la cola por identificador y no por posición: entre que se abre
	 * el menú y se elige «quitar», la canción que termina puede haber corrido
	 * toda la lista un lugar hacia arriba.
	 */
	const removeQueueItem = (id: string) => {
		queueEntries.value = removeQueueEntry(queueEntries.value, id);
	};

	/** Reordenar, también por identificador y por el mismo motivo. */
	const moveQueueItem = (fromId: string, toId: string) => {
		queueEntries.value = moveQueueEntry(queueEntries.value, fromId, toId);
	};

	/** La entrada señalada, o nada si ya no está en la cola. */
	const getQueueEntry = (id: string) => findQueueEntry(queueEntries.value, id);

	const handleDroppedPaths = async (paths: string[]) => {
		devLog('[handleDroppedPaths] Paths recibidos:', paths);
		const normalized = Array.from(
			new Set(paths.map((path) => path.trim()).filter((path) => path.length > 0))
		);
		devLog('[handleDroppedPaths] Paths normalizados:', normalized);
		const [firstPath, ...rest] = normalized;
		if (!firstPath) {
			devLog('[handleDroppedPaths] No hay primera ruta para reproducir');
			return;
		}

		queueEntries.value = createQueueEntries(rest);
		void ensureMetadataForPaths(normalized);
		devLog('[handleDroppedPaths] Llamando playDropped con:', firstPath);
		await playDropped(firstPath);
	};

	const setScanning = (value: boolean) => {
		isScanning.value = value;
	};

	const reloadLibraryTracks = async () => {
		try {
			// Limpiar cache local primero para forzar recarga desde cero
			trackCacheByPath.value = {};
			persistTrackCache();

			const tracks = await listLibraryTracks();
			const newCache: Record<string, DroppedPlaybackTrack> = {};

			// Extraer metadatos visuales (cover, color) de todos los tracks en batches
			const batchSize = 20;
			const visualMetadata: {
				path: string;
				cover_data_url: string | null;
				dominant_color: string | null;
			}[] = [];

			for (let i = 0; i < tracks.length; i += batchSize) {
				const batch = tracks.slice(i, i + batchSize);
				const batchResults = await Promise.allSettled(
					batch.map(async (track) => {
						try {
							const fullMetadata = await handleDroppedFile(track.path);
							return {
								path: track.path,
								cover_data_url: fullMetadata.cover_data_url,
								dominant_color: fullMetadata.dominant_color,
							};
						} catch {
							return {
								path: track.path,
								cover_data_url: null,
								dominant_color: null,
							};
						}
					})
				);
				for (const result of batchResults) {
					if (result.status === 'fulfilled') {
						visualMetadata.push(result.value);
					}
				}
			}

			// Crear mapa de metadatos visuales por path
			const visualMap: Record<
				string,
				{ cover_data_url: string | null; dominant_color: string | null }
			> = {};
			for (const item of visualMetadata) {
				visualMap[item.path] = item;
			}

			// Reconstruir cache combinando datos de BD con metadatos visuales
			for (const track of tracks) {
				const newVisuals = visualMap[track.path];

				newCache[track.path] = {
					path: track.path,
					title: track.title,
					artist: track.artist,
					album: track.album,
					duration_seconds: track.duration_seconds,
					cover_data_url: newVisuals?.cover_data_url ?? null,
					dominant_color: newVisuals?.dominant_color ?? null,
				};
			}

			trackCacheByPath.value = newCache;
			persistTrackCache();
		} catch (err) {
			console.error('[reloadLibraryTracks] Error loading tracks:', err);
		}
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

		queueEntries.value = [...queueEntries.value, ...createQueueEntries(nextItems)];
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

		const existingAfterAlbum = queueEntries.value.filter(
			(entry) => !restPaths.includes(entry.path)
		);
		queueEntries.value = [...createQueueEntries(restPaths), ...existingAfterAlbum];
		void ensureMetadataForPaths(normalized);
		await playDropped(firstPath);
	};

	const advanceQueue = async () => {
		await advancePlayback();
	};

	syncTrackCacheFromStorage();
	syncFavoritesFromStorage();
	void ensureMetadataForFavorites();

	// Restaurar estado de reproducción guardado y persistir cambios
	void syncPlaybackStateFromStorage();

	let persistTimer: ReturnType<typeof setTimeout> | null = null;
	const debouncedPersist = () => {
		if (persistTimer) {
			clearTimeout(persistTimer);
		}
		persistTimer = setTimeout(() => {
			void persistPlaybackState();
			persistTimer = null;
		}, 1000);
	};

	watch(
		[currentPath, queue, isPlaying, volume],
		() => {
			debouncedPersist();
		},
		{ deep: false }
	);

	watch(positionSeconds, () => {
		debouncedPersist();
	});

	if (typeof window !== 'undefined') {
		beforeUnloadHandler = () => {
			void persistPlaybackState();
		};
		window.addEventListener('beforeunload', beforeUnloadHandler);
		void (async () => {
			try {
				await listen('tauri://close-requested', () => {
					void persistPlaybackState();
				});
			} catch {
				// ignore if not running under Tauri or listen fails
			}
		})();
	}

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
		syncPlaybackSnapshot,
		initMprisNextListener,
		initMprisPreviousListener,
		initMprisStopListener,
		favoritePaths,
		favoriteEntries,
		globalBadgeMessage,
		getTrackMetadata,
		enqueuePaths,
		showGlobalBadge,
		playAlbum,
		isCurrentFavorite,
		isFavoritePath,
		isScanning,
		queue,
		queueEntries,
		queuedCount,
		getQueueEntry,
		nextActionLabel,
		nextSuggestionPath,
		trackCacheList,
		ensureMetadataForFavorites,
		ensureMetadataForPath,
		setCurrentTrackVisuals,
		moveQueueItem,
		removeQueueItem,
		reloadLibraryTracks,
		setScanning,
		isDragOver,
		isPaused,
		isStream,
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
