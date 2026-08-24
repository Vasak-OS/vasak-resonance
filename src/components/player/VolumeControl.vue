<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import { usePlayerStore } from '@/stores/player';

const { t } = useI18n();
const playerStore = usePlayerStore();

const volumePercent = computed(() => Math.round((playerStore.volume || 0) * 50));

const onInput = (e: Event) => {
	const target = e.target as HTMLInputElement;
	const val = Number(target.value);
	// slider 0..100 -> normalize to 0..2 (same as store expects)
	const normalized = Math.max(0, Math.min(100, val)) / 50;
	void playerStore.setVolume(normalized);
};
</script>

<template>
	<div class="hidden md:flex items-center gap-2">
		<button class="inline-flex items-center gap-2 rounded-corner border border-ui-border bg-ui-surface/55 px-2 py-1 text-xs text-tx-main" :title="t('volume.label')">
			<svg class="h-4 w-4 text-tx-muted" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
				<path d="M9 4.5v11l-4-3H3a1 1 0 01-1-1v-3a1 1 0 011-1h2l4-3z"></path>
			</svg>
			<span class="text-xs text-tx-muted">{{ volumePercent }}%</span>
		</button>
		<input type="range" min="0" max="100" :value="Math.round((playerStore.volume||0)*50)" @input="onInput" class="h-1 w-36 appearance-none rounded bg-ui-border/40 accent-primary/70" />
	</div>
</template>

<style scoped>
input[type="range"]::-webkit-slider-thumb {
	-webkit-appearance: none;
	width: 14px;
	height: 14px;
	border-radius: 99px;
	background: var(--color-primary);
	box-shadow: 0 0 0 3px rgba(0,0,0,0.05);
}
</style>
