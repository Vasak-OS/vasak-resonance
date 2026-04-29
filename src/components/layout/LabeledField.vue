<script setup lang="ts">
withDefaults(
	defineProps<{
		label: string;
		wrapperClass?: string;
	}>(),
	{
		wrapperClass: '',
	}
);
</script>

<template>
	<label :class="['grid gap-1.5 text-xs uppercase tracking-[0.14em] text-tx-muted', wrapperClass]">
		<span class="px-0.5">{{ label }}</span>
		<slot />
	</label>
</template>
