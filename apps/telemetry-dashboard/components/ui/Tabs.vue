<script setup lang="ts">
defineProps<{
	modelValue: string
	label: string
	items: Array<{ value: string; label: string }>
}>()

defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
	<div
		role="tablist"
		:aria-label="label"
		class="inline-flex h-9 items-center rounded-lg border border-surface-4 bg-surface-3 p-1 text-muted-foreground"
	>
		<button
			v-for="item in items"
			:key="item.value"
			type="button"
			role="tab"
			:aria-selected="modelValue === item.value"
			class="inline-flex h-7 cursor-pointer items-center justify-center whitespace-nowrap rounded-md px-2.5 text-xs font-medium transition-all hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
			:class="modelValue === item.value && 'bg-surface-2 text-foreground shadow-sm'"
			@click="$emit('update:modelValue', item.value)"
		>
			{{ item.label }}
		</button>
	</div>
</template>
