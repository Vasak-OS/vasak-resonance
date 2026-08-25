import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { setCrossfade } from '@/services/player.service';

/** Where the settings live on disk, next to `resonance-playback.json`. */
const STORE_FILE = 'resonance-settings.json';
const STORE_KEY = 'settings';
/** Fallback when the Tauri store plugin is unavailable, as the player store does. */
const LOCAL_STORAGE_KEY = 'resonance.settings';

/** Matches `DEFAULT_CROSSFADE` in `src-tauri/src/audio_manager.rs`. */
export const DEFAULT_CROSSFADE_SECONDS = 4;
/** Matches `MAX_CROSSFADE`. Asking for more is clamped by the backend anyway. */
export const MAX_CROSSFADE_SECONDS = 12;
/**
 * Below this an overlap is not a transition, it is a click. Anyone who wants
 * less wants none, which is what the switch is for.
 */
export const MIN_CROSSFADE_SECONDS = 1;

interface PersistedSettings {
	crossfadeEnabled?: boolean;
	crossfadeSeconds?: number;
}

/**
 * The slice of the Tauri store plugin this file uses.
 *
 * Declared here rather than imported: the plugin is loaded dynamically so it can
 * be absent, and naming only the three methods needed keeps that optional import
 * from leaking a type dependency into the build.
 */
interface SettingsStorage {
	get(key: string): Promise<PersistedSettings | null | undefined>;
	set(key: string, value: PersistedSettings): Promise<void>;
	save(): Promise<void>;
}

export const useSettingsStore = defineStore('settings', () => {
	const crossfadeEnabled = ref(true);
	/**
	 * Kept separately from `crossfadeEnabled` so turning the overlap off and on
	 * again returns to the length that was chosen, rather than to the default.
	 */
	const crossfadeSeconds = ref(DEFAULT_CROSSFADE_SECONDS);
	/**
	 * Guards against loading twice.
	 *
	 * Holds the in-flight promise rather than a boolean: Vue mounts children
	 * before parents, so the settings view's `onMounted` runs before `App`'s, and
	 * a boolean checked after an `await` lets both calls through.
	 */
	let loading: Promise<void> | null = null;

	const storage = ref<SettingsStorage | null>(null);

	/** What the backend is told: zero means "do not overlap". */
	const effectiveCrossfade = computed(() => (crossfadeEnabled.value ? crossfadeSeconds.value : 0));

	const clampSeconds = (value: number): number => {
		if (!Number.isFinite(value)) {
			return DEFAULT_CROSSFADE_SECONDS;
		}
		return Math.min(MAX_CROSSFADE_SECONDS, Math.max(MIN_CROSSFADE_SECONDS, Math.round(value)));
	};

	const initStorage = async () => {
		if (storage.value) {
			return;
		}
		try {
			const plugin = await import('@tauri-apps/plugin-store');
			storage.value = await new plugin.LazyStore(STORE_FILE);
		} catch {
			// No plugin: localStorage below carries the settings instead.
		}
	};

	const readPersisted = async (): Promise<PersistedSettings | null> => {
		await initStorage();
		if (storage.value) {
			return (await storage.value.get(STORE_KEY)) ?? null;
		}

		if (typeof window === 'undefined') {
			return null;
		}
		const raw = window.localStorage.getItem(LOCAL_STORAGE_KEY);
		if (!raw) {
			return null;
		}
		try {
			return JSON.parse(raw) as PersistedSettings;
		} catch {
			return null;
		}
	};

	const persist = async () => {
		const payload: PersistedSettings = {
			crossfadeEnabled: crossfadeEnabled.value,
			crossfadeSeconds: crossfadeSeconds.value,
		};
		try {
			await initStorage();
			if (storage.value) {
				await storage.value.set(STORE_KEY, payload);
				await storage.value.save();
				return;
			}
			if (typeof window !== 'undefined') {
				window.localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(payload));
			}
		} catch (persistError) {
			console.error('[settings] no se pudieron guardar los ajustes:', persistError);
		}
	};

	/**
	 * Pushes the current value to the audio thread.
	 *
	 * The backend holds its own default, so a failure here leaves playback
	 * working with the built-in four seconds rather than broken.
	 */
	const applyCrossfade = async () => {
		try {
			await setCrossfade(effectiveCrossfade.value);
		} catch (applyError) {
			console.error('[settings] no se pudo aplicar el encadenado:', applyError);
		}
	};

	/**
	 * Reads the saved settings and hands them to the backend.
	 *
	 * Called once at startup: the audio thread starts on its own default, and
	 * without this a person who turned the overlap off would hear it again on
	 * every launch.
	 */
	const load = (): Promise<void> => {
		if (!loading) {
			loading = (async () => {
				const persisted = await readPersisted();
				if (persisted) {
					if (typeof persisted.crossfadeEnabled === 'boolean') {
						crossfadeEnabled.value = persisted.crossfadeEnabled;
					}
					if (typeof persisted.crossfadeSeconds === 'number') {
						crossfadeSeconds.value = clampSeconds(persisted.crossfadeSeconds);
					}
				}
				await applyCrossfade();
			})();
		}
		return loading;
	};

	const setCrossfadeEnabled = async (enabled: boolean) => {
		crossfadeEnabled.value = enabled;
		await applyCrossfade();
		await persist();
	};

	const setCrossfadeSeconds = async (seconds: number) => {
		crossfadeSeconds.value = clampSeconds(seconds);
		await applyCrossfade();
		await persist();
	};

	const resetCrossfade = async () => {
		crossfadeEnabled.value = true;
		crossfadeSeconds.value = DEFAULT_CROSSFADE_SECONDS;
		await applyCrossfade();
		await persist();
	};

	return {
		crossfadeEnabled,
		crossfadeSeconds,
		effectiveCrossfade,
		load,
		setCrossfadeEnabled,
		setCrossfadeSeconds,
		resetCrossfade,
	};
});
