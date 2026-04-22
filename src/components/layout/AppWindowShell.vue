<script setup lang="ts">
defineProps<{
	contentClass?: string;
}>();
</script>

<template>
	<div
		class="flex h-screen w-screen flex-col overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/80"
		:class="contentClass"
	>
		<slot />
	</div>
</template>
