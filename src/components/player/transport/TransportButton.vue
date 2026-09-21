<script setup lang="ts">
/**
 * Un botón del transporte: reproducir, siguiente, repetir, aleatorio.
 *
 * El icono se pide por **nombre** y no por ruta. Antes recibía una ruta ya
 * resuelta —`iconSrc`— y quien lo usaba tenía que resolverla, escuchar el
 * cambio de tema y volver a pedirla; eso es lo que hacía `useReactiveIcon`, y
 * es lo que `ThemeIcon` hace una sola vez para todo el escritorio.
 *
 * El nombre del botón vive en `aria-label` y en `title`, así que el icono no
 * lleva texto propio: repetirlo hace que un lector de pantalla lo diga dos
 * veces.
 */
import { ThemeIcon } from '@vasakgroup/vue-libvasak';

const props = withDefaults(
	defineProps<{
		label: string;
		disabled?: boolean;
		variant?: 'primary' | 'secondary';
		size?: 'sm' | 'md';
		/** Nombre del icono en el tema del escritorio, no una ruta. */
		icon?: string;
		showLabel?: boolean;
	}>(),
	{
		disabled: false,
		variant: 'secondary',
		size: 'md',
		icon: '',
		showLabel: false,
	}
);

const emit = defineEmits<{
	click: [];
}>();

const onClick = () => {
	if (props.disabled) {
		return;
	}
	emit('click');
};
</script>

<template>
	<button
		type="button"
		:disabled="disabled"
		:title="label"
		:aria-label="label"
		class="rounded-corner border font-semibold transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-50"
		:class="[
			size === 'sm' ? 'px-2 py-1.5 text-[11px]' : 'px-2 py-2 text-xs',
			variant === 'primary'
				? 'border-primary/45 bg-primary text-tx-on-primary hover:bg-primary/90'
				: 'border-ui-border bg-ui-bg/50 text-tx-main hover:bg-ui-surface/80',
		]"
		@click="onClick"
	>
		<span class="flex items-center justify-center gap-1">
			<ThemeIcon v-if="icon" :name="icon" type="symbol" :size="16" />
			<span v-if="showLabel || !icon">{{ label }}</span>
		</span>
	</button>
</template>
