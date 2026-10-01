<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Checkbox,
	ConfigSection,
	PageHeader,
	SettingRow,
	Slider,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import {
	finishLastfmAuthorization,
	getLastfmStatus,
	type LastfmStatus,
	startLastfmAuthorization,
	unlinkLastfm,
} from '@/services/lastfm.service';
import { MAX_CROSSFADE_SECONDS, MIN_CROSSFADE_SECONDS, useSettingsStore } from '@/stores/settings';
import { lastfmStep } from '@/tools/lastfm';

const { t } = useI18n();
const settings = useSettingsStore();

const lastfm = ref<LastfmStatus>({ configured: false, user: null });
const pendingToken = ref<string | null>(null);
const lastfmError = ref<string | null>(null);

/**
 * Si hay una operación de Last.fm en curso.
 *
 * Sin esto, dos clics seguidos en «Vincular» son dos pedidos y dos pestañas del
 * navegador, y el token que vuelve segundo pisa al primero: la persona autoriza
 * uno y se confirma el otro, que nadie autorizó. Se pone antes del primer
 * `await`, así el segundo clic del mismo tic ya lo ve puesto.
 */
const lastfmBusy = ref(false);

const step = computed(() => lastfmStep(lastfm.value, pendingToken.value));

const linkedAs = computed(() =>
	t('settings.lastfmLinkedAs').replace('{0}', lastfm.value.user ?? '')
);

onMounted(() => {
	void settings.load();
	void reloadLastfm();
});

async function reloadLastfm() {
	lastfm.value = await getLastfmStatus();
}

async function onLink() {
	if (lastfmBusy.value) {
		return;
	}

	lastfmBusy.value = true;
	lastfmError.value = null;
	try {
		pendingToken.value = await startLastfmAuthorization();
	} catch (error) {
		lastfmError.value = String(error);
	} finally {
		lastfmBusy.value = false;
	}
}

async function onConfirm() {
	const token = pendingToken.value;
	if (!token || lastfmBusy.value) {
		return;
	}

	lastfmBusy.value = true;
	lastfmError.value = null;
	try {
		lastfm.value = await finishLastfmAuthorization(token);
		// Sólo al salir bien: si la persona todavía no autorizó allá, esto falla
		// y tiene que poder volver a apretar sin empezar la vuelta de nuevo.
		pendingToken.value = null;
	} catch (error) {
		lastfmError.value = String(error);
	} finally {
		lastfmBusy.value = false;
	}
}

function onCancel() {
	pendingToken.value = null;
	lastfmError.value = null;
}

async function onUnlink() {
	if (lastfmBusy.value) {
		return;
	}

	lastfmBusy.value = true;
	lastfmError.value = null;
	try {
		lastfm.value = await unlinkLastfm();
	} catch (error) {
		lastfmError.value = String(error);
	} finally {
		lastfmBusy.value = false;
	}
}

const secondsLabel = computed(() =>
	t(settings.crossfadeSeconds === 1 ? 'settings.secondsOne' : 'settings.secondsOther').replace(
		'{0}',
		String(settings.crossfadeSeconds)
	)
);

const onToggle = (checked: boolean) => {
	void settings.setCrossfadeEnabled(checked);
};

// Se guarda al soltar (`lazy`), no en cada paso del arrastre: cada valor es una
// escritura en disco y un aviso al hilo de audio.
const onSeconds = (seconds: number) => {
	void settings.setCrossfadeSeconds(seconds);
};
</script>

<template>
	<div class="flex h-full flex-col gap-4 overflow-y-auto px-4 py-4">
		<PageHeader :eyebrow="t('settings.eyebrow')" :title="t('settings.title')" size="lg" />

		<section :aria-label="t('settings.playbackGroup')">
		<ConfigSection :title="t('settings.playbackGroup')" as="h2" class="gap-4">
			<!-- Una casilla, como antes: el encadenado se prende o se apaga. -->
			<SettingRow :label="t('settings.crossfade')" :description="t('settings.crossfadeHint')" control-id="crossfade-enabled">
				<Checkbox
					id="crossfade-enabled"
					:model-value="settings.crossfadeEnabled"
					:label="t('settings.crossfade')"
					hide-label
					@change="onToggle"
				/>
			</SettingRow>

			<!-- The slider is disabled rather than hidden: someone who turns the
			     overlap off should still see the length it will come back with. -->
			<div class="grid gap-1.5" :class="settings.crossfadeEnabled ? '' : 'opacity-50'">
				<span class="flex items-baseline justify-between gap-2">
					<span class="text-xs uppercase tracking-[0.14em] text-tx-muted">
						{{ t('settings.crossfadeLength') }}
					</span>
					<span class="text-sm font-medium text-tx-main">{{ secondsLabel }}</span>
				</span>
				<Slider
					:model-value="settings.crossfadeSeconds"
					:min="MIN_CROSSFADE_SECONDS"
					:max="MAX_CROSSFADE_SECONDS"
					:step="1"
					:label="t('settings.crossfadeLength')"
					:value-text="secondsLabel"
					:disabled="!settings.crossfadeEnabled"
					lazy
					@change="onSeconds"
				/>
			</div>

			<p class="text-xs text-tx-muted">{{ t('settings.crossfadeSegueWarning') }}</p>

			<template #actions>
				<ActionButton :label="t('settings.restoreDefaults')" variant="secondary" size="sm" @click="settings.resetCrossfade()" />
			</template>
		</ConfigSection>
		</section>

		<!-- Sin clave de API no se dibuja nada: es el caso de casi todo el mundo,
		     y una sección que promete vincular algo que no se puede vincular es
		     peor que no tenerla. Igual que la presencia en Discord. -->
		<section v-if="step !== 'hidden'" :aria-label="t('settings.lastfmGroup')">
		<ConfigSection :title="t('settings.lastfmGroup')" :description="t('settings.lastfmHint')" as="h2" class="gap-3">
			<div v-if="step === 'linked'" class="flex min-w-0 flex-wrap items-center justify-between gap-4">
				<span class="min-w-0 break-words text-sm text-tx-main">{{ linkedAs }}</span>
				<ActionButton
					:label="t('settings.lastfmUnlink')"
					variant="secondary"
					size="sm"
					:disabled="lastfmBusy"
					@click="onUnlink"
				/>
			</div>

			<div v-else-if="step === 'authorizing'" class="grid gap-2">
				<p class="text-sm text-tx-main">{{ t('settings.lastfmAuthorizing') }}</p>
				<div class="flex flex-wrap justify-end gap-2">
					<ActionButton :label="t('settings.lastfmCancel')" variant="secondary" size="sm" @click="onCancel" />
					<ActionButton :label="t('settings.lastfmConfirm')" size="sm" :disabled="lastfmBusy" @click="onConfirm" />
				</div>
			</div>

			<div v-else class="flex justify-end">
				<ActionButton :label="t('settings.lastfmLink')" size="sm" :disabled="lastfmBusy" @click="onLink" />
			</div>

			<AlertMessage v-if="lastfmError" tone="error" icon="auto">
				{{ t('settings.lastfmError').replace('{0}', lastfmError) }}
			</AlertMessage>
		</ConfigSection>
		</section>
	</div>
</template>
