<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import {
	desvincularLastfm,
	type EstadoDeLastfm,
	empezarAutorizacion,
	estadoDeLastfm,
	terminarAutorizacion,
} from '@/services/lastfm.service';
import { MAX_CROSSFADE_SECONDS, MIN_CROSSFADE_SECONDS, useSettingsStore } from '@/stores/settings';
import { pasoDeLastfm } from '@/tools/lastfm';

const { t } = useI18n();
const settings = useSettingsStore();

const lastfm = ref<EstadoDeLastfm>({ configurado: false, usuario: null });
const tokenPendiente = ref<string | null>(null);
const errorDeLastfm = ref<string | null>(null);

/**
 * Si hay una operación de Last.fm en curso.
 *
 * Sin esto, dos clics seguidos en «Vincular» son dos pedidos y dos pestañas del
 * navegador, y el token que vuelve segundo pisa al primero: la persona autoriza
 * uno y se confirma el otro, que nadie autorizó. Se pone antes del primer
 * `await`, así el segundo clic del mismo tic ya lo ve puesto.
 */
const lastfmOcupado = ref(false);

const pasoLastfm = computed(() => pasoDeLastfm(lastfm.value, tokenPendiente.value));

const vinculadoComo = computed(() =>
	t('settings.lastfmLinkedAs').replace('{0}', lastfm.value.usuario ?? '')
);

onMounted(() => {
	void settings.load();
	void recargarLastfm();
});

async function recargarLastfm() {
	lastfm.value = await estadoDeLastfm();
}

async function onVincular() {
	if (lastfmOcupado.value) {
		return;
	}

	lastfmOcupado.value = true;
	errorDeLastfm.value = null;
	try {
		tokenPendiente.value = await empezarAutorizacion();
	} catch (error) {
		errorDeLastfm.value = String(error);
	} finally {
		lastfmOcupado.value = false;
	}
}

async function onConfirmar() {
	const token = tokenPendiente.value;
	if (!token || lastfmOcupado.value) {
		return;
	}

	lastfmOcupado.value = true;
	errorDeLastfm.value = null;
	try {
		lastfm.value = await terminarAutorizacion(token);
		// Sólo al salir bien: si la persona todavía no autorizó allá, esto falla
		// y tiene que poder volver a apretar sin empezar la vuelta de nuevo.
		tokenPendiente.value = null;
	} catch (error) {
		errorDeLastfm.value = String(error);
	} finally {
		lastfmOcupado.value = false;
	}
}

function onCancelar() {
	tokenPendiente.value = null;
	errorDeLastfm.value = null;
}

async function onDesvincular() {
	if (lastfmOcupado.value) {
		return;
	}

	lastfmOcupado.value = true;
	errorDeLastfm.value = null;
	try {
		lastfm.value = await desvincularLastfm();
	} catch (error) {
		errorDeLastfm.value = String(error);
	} finally {
		lastfmOcupado.value = false;
	}
}

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

		<!-- Sin clave de API no se dibuja nada: es el caso de casi todo el mundo,
		     y una sección que promete vincular algo que no se puede vincular es
		     peor que no tenerla. Igual que la presencia en Discord. -->
		<section
			v-if="pasoLastfm !== 'oculto'"
			class="grid gap-3 rounded-corner border border-ui-border bg-ui-surface/55 p-4"
			:aria-label="t('settings.lastfmGroup')"
		>
			<h2 class="text-sm font-semibold text-tx-main">{{ t('settings.lastfmGroup') }}</h2>
			<p class="text-xs text-tx-muted">{{ t('settings.lastfmHint') }}</p>

			<div v-if="pasoLastfm === 'vinculado'" class="flex items-center justify-between gap-4">
				<span class="text-sm text-tx-main">{{ vinculadoComo }}</span>
				<button
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
					:disabled="lastfmOcupado"
					@click="onDesvincular"
				>
					{{ t('settings.lastfmUnlink') }}
				</button>
			</div>

			<div v-else-if="pasoLastfm === 'autorizando'" class="grid gap-2">
				<p class="text-sm text-tx-main">{{ t('settings.lastfmAuthorizing') }}</p>
				<div class="flex justify-end gap-2">
					<button
						type="button"
						class="rounded-corner border border-ui-border bg-ui-surface/55 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:border-primary/40 hover:bg-ui-surface/75"
						@click="onCancelar"
					>
						{{ t('settings.lastfmCancel') }}
					</button>
					<button
						type="button"
						class="rounded-corner border border-primary/40 bg-primary/15 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:bg-primary/25"
						:disabled="lastfmOcupado"
						@click="onConfirmar"
					>
						{{ t('settings.lastfmConfirm') }}
					</button>
				</div>
			</div>

			<div v-else class="flex justify-end">
				<button
					type="button"
					class="rounded-corner border border-primary/40 bg-primary/15 px-3 py-1.5 text-xs font-medium text-tx-main transition-colors duration-200 hover:bg-primary/25"
					:disabled="lastfmOcupado"
					@click="onVincular"
				>
					{{ t('settings.lastfmLink') }}
				</button>
			</div>

			<p v-if="errorDeLastfm" class="text-xs text-red-400" role="alert">
				{{ t('settings.lastfmError').replace('{0}', errorDeLastfm) }}
			</p>
		</section>
	</div>
</template>
