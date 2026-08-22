<script setup lang="ts">
withDefaults(
	defineProps<{
		compact?: boolean
		stacked?: boolean
	}>(),
	{
		compact: false,
		stacked: false,
	},
)
</script>

<template>
	<div
		class="settings-row"
		:class="{ 'settings-row-compact': compact, 'settings-row-stacked': stacked }"
	>
		<div class="settings-row-copy">
			<div v-if="$slots.label" class="settings-row-label">
				<slot name="label" />
			</div>
			<div v-if="$slots.description" class="settings-row-description">
				<slot name="description" />
			</div>
			<slot name="copy" />
		</div>
		<div v-if="$slots.control" class="settings-row-control">
			<slot name="control" />
		</div>
	</div>
</template>

<style scoped>
.settings-row {
	display: grid;
	grid-template-columns: minmax(0, 1fr) minmax(10rem, 12rem);
	align-items: center;
	gap: var(--gap-xl);
	min-height: 4rem;
	padding: var(--gap-md) var(--gap-lg);
	border-bottom: 1px solid
		var(--settings-divider, color-mix(in srgb, var(--surface-4) 55%, transparent));
}

.settings-row:last-child {
	border-bottom: 0;
}

.settings-row-compact {
	min-height: 3.5rem;
}

.settings-row-stacked {
	grid-template-columns: minmax(0, 1fr);
	align-items: start;
	gap: var(--gap-md);
}

.settings-row-copy {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: var(--gap-xs);
}

.settings-row-label {
	color: var(--color-contrast);
	font-size: 1rem;
	font-weight: 600;
}

.settings-row-description {
	color: var(--color-secondary);
	font-size: 0.875rem;
	line-height: 1.45;
}

.settings-row-control {
	display: flex;
	min-width: 0;
	justify-content: flex-end;
}

.settings-row-stacked .settings-row-control {
	justify-content: flex-start;
	width: 100%;
}

.settings-row-control :deep(.btn),
.settings-row-control :deep(input),
.settings-row-control :deep(select),
.settings-row-control :deep(.combobox) {
	max-width: 100%;
}

@media (max-width: 700px) {
	.settings-row {
		grid-template-columns: minmax(0, 1fr);
		align-items: start;
		gap: var(--gap-md);
	}

	.settings-row-control {
		justify-content: flex-start;
		width: 100%;
	}
}
</style>
