import { computed, ref, type Ref } from 'vue';
import { getOrFetchCoverUrl } from '@/services/album-cover.service';

export interface UseAlbumCoverOptions {
	artist: () => string;
	album: () => string;
	existingCover?: () => string | null | undefined;
}

/**
 * Composable for managing album cover images
 * Handles fetching from cache/APIs when needed
 */
export function useAlbumCover(options: UseAlbumCoverOptions) {
	const coverUrl: Ref<string> = ref('');
	const isLoading: Ref<boolean> = ref(false);
	const error: Ref<string> = ref('');

	const fetchCover = async () => {
		if (!options.artist() || !options.album()) {
			coverUrl.value = '';
			return;
		}

		isLoading.value = true;
		error.value = '';

		try {
			const existing = options.existingCover?.();
			const url = await getOrFetchCoverUrl(options.artist(), options.album(), existing);
			coverUrl.value = url;
		} catch (err) {
			error.value = err instanceof Error ? err.message : String(err);
			coverUrl.value = '';
		} finally {
			isLoading.value = false;
		}
	};

	const displayUrl = computed(() => {
		return coverUrl.value || (options.existingCover?.() ?? '');
	});

	return {
		coverUrl,
		displayUrl,
		isLoading,
		error,
		fetchCover,
	};
}
