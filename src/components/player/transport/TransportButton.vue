<script setup lang="ts">
const props = withDefaults(
	defineProps<{
		label: string;
		disabled?: boolean;
		variant?: 'primary' | 'secondary';
		size?: 'sm' | 'md';
		iconSrc?: string;
		iconAlt?: string;
		showLabel?: boolean;
	}>(),
	{
		disabled: false,
		variant: 'secondary',
		size: 'md',
		iconSrc: '',
		iconAlt: '',
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
			<img v-if="iconSrc" :src="iconSrc" :alt="iconAlt || label" class="h-4 w-4 object-contain">
			<span v-if="showLabel || !iconSrc">{{ label }}</span>
		</span>
	</button>
</template>
