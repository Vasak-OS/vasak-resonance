<script setup lang="ts">
const props = withDefaults(
	defineProps<{
		label: string;
		disabled?: boolean;
		variant?: 'primary' | 'secondary';
		size?: 'sm' | 'md';
	}>(),
	{
		disabled: false,
		variant: 'secondary',
		size: 'md',
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
		class="rounded-corner border font-semibold transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-50"
		:class="[
			size === 'sm' ? 'px-2 py-1.5 text-[11px]' : 'px-2 py-2 text-xs',
			variant === 'primary'
				? 'border-primary/45 bg-primary text-tx-on-primary hover:bg-primary/90'
				: 'border-ui-border bg-ui-bg/50 text-tx-main hover:bg-ui-surface/80',
		]"
		@click="onClick"
	>
		{{ label }}
	</button>
</template>
