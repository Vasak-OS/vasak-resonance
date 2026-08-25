<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted } from 'vue';
import { MAX_CROSSFADE_SECONDS, MIN_CROSSFADE_SECONDS, useSettingsStore } from '@/stores/settings';

const { t } = useI18n();
const settings = useSettingsStore();

onMounted(() => {
	void settings.load();
});

const secondsLabel = computed(() =>
	t(settings.crossfadeSeconds === 1 ? 'settings.secondsOne' : 'settings.secondsOther').replace(
		'{0}',
		String(settings.crossfadeSeconds)
	)
);

const onToggle = (event: Event) => {
	void settings.setCrossfadeEnabled((event.target as HTMLInputElement).checked);
};

const onSeconds = (event: Event) => {
	void settings.setCrossfadeSeconds(Number((event.target as HTMLInputElement).value));
};
</script>

<template>
	<div class="flex h-full flex-col gap-4 overflow-y-auto px-4 py-4">
		<div>
			<p class="text-xs uppercase tracking-[0.16em] text-tx-muted">
				{{ t('settings.eyebrow') }}
			</p>
			<h1 class="text-2xl font-bold text-tx-main">{{ t('settings.title') }}</h1>
		</div>

		<section
			class="grid gap-4 rounded-corner border border-ui-border bg-ui-surface/55 p-4"
			:aria-label="t('settings.playbackGroup')"
		>
			<h2 class="text-sm font-semibold text-tx-main">{{ t('settings.playbackGroup') }}</h2>

			<label class="flex items-start justify-between gap-4">
				<span class="grid gap-1">
					<span class="text-sm font-medium text-tx-main">{{ t('settings.crossfade') }}</span>
					<span class="text-xs text-tx-muted">{{ t('settings.crossfadeHint') }}</span>
				</span>
				<input
					type="checkbox"
					class="mt-1 h-4 w-4 shrink-0 accent-primary"
					:checked="settings.crossfadeEnabled"
					@change="onToggle"
				>
			</label>

			<!-- The slider is disabled rather than hidden: someone who turns the
			     overlap off should still see the length it will come back with. -->
			<label class="grid gap-1.5" :class="settings.crossfadeEnabled ? '' : 'opacity-50'">
				<span class="flex items-baseline justify-between gap-2">
					<span class="text-xs uppercase tracking-[0.14em] text-tx-muted">
						{{ t('settings.crossfadeLength') }}
					</span>
					<span class="text-sm font-medium text-tx-main">{{ secondsLabel }}</span>
				</span>
				<input
					type="range"
					class="w-full accent-primary"
					:min="MIN_CROSSFADE_SECONDS"
					:max="MAX_CROSSFADE_SECONDS"
					step="1"
					:value="settings.crossfadeSeconds"
					:disabled="!settings.crossfadeEnabled"
					:aria-label="t('settings.crossfadeLength')"
					@change="onSeconds"
				>
			</label>

			<p class="text-xs text-tx-muted">{{ t('settings.crossfadeSegueWarning') }}</p>

			<div class="flex justify-end">
				<button
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					@click="settings.resetCrossfade()"
				>
					{{ t('settings.restoreDefaults') }}
				</button>
			</div>
		</section>
	</div>
</template>
