<!-- 由 S4 集成到 LabRecipeGenerator.vue -->
<script setup lang="ts">
import { defineMessages, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { SlotDisplay } from '@/lab/recipe-generator/display'
import type { TextureAtlas } from '@/lab/recipe-generator/resources'
import type { RecipeSlot, SlotValue } from '@/lab/recipe-generator/types'

import RecipeItemIcon from './RecipeItemIcon.vue'

const RECIPE_SLOT_MIME_TYPE = 'application/x-axolotl-recipe-slot'

const props = withDefaults(
	defineProps<{
		slot: RecipeSlot
		value: SlotValue | undefined
		display: SlotDisplay | null
		atlas: TextureAtlas
		count?: number
		countEditable?: boolean
		result?: boolean
	}>(),
	{
		count: 1,
		countEditable: false,
		result: false,
	},
)

const emit = defineEmits<{
	clear: []
	dropValue: [value: SlotValue]
	updateCount: [count: number]
}>()

const { formatMessage } = useVIntl()
const dragDepth = ref(0)

const messages = defineMessages({
	emptySlot: { id: 'app.lab.recipe-generator.slots.empty', defaultMessage: 'Empty slot' },
	resultCount: {
		id: 'app.lab.recipe-generator.slots.result-count',
		defaultMessage: 'Result count',
	},
})

const dragActive = computed(() => dragDepth.value > 0)
const showCountField = computed(() => props.countEditable)
const slotLabel = computed(() => `${formatMessage(messages.emptySlot)} ${props.slot}`)

function hasRecipePayload(event: DragEvent) {
	const types = event.dataTransfer?.types
	if (!types) return false
	const typeList = Array.from(types)
	return (
		!typeList.includes('Files') &&
		(typeList.includes(RECIPE_SLOT_MIME_TYPE) || typeList.includes('text/plain'))
	)
}

function isSlotValue(value: unknown): value is SlotValue {
	if (typeof value !== 'object' || value === null) return false
	const candidate = value as { kind?: unknown; id?: unknown; uid?: unknown }
	switch (candidate.kind) {
		case 'item':
		case 'vanilla_tag':
			return typeof candidate.id === 'string'
		case 'custom_item':
		case 'custom_tag':
			return typeof candidate.uid === 'string'
		default:
			return false
	}
}

function parseSlotValue(raw: string): SlotValue | null {
	if (!raw) return null
	try {
		const parsed: unknown = JSON.parse(raw)
		return isSlotValue(parsed) ? parsed : null
	} catch {
		return null
	}
}

function onSlotDropEvent(event: Event) {
	const detail = (event as CustomEvent<{ value?: unknown }>).detail
	if (!detail || !isSlotValue(detail.value)) return
	emit('dropValue', detail.value)
}

function onDragEnter(event: DragEvent) {
	if (!hasRecipePayload(event)) return
	dragDepth.value += 1
}

function onDragOver(event: DragEvent) {
	const dataTransfer = event.dataTransfer
	if (!dataTransfer) return
	if (!hasRecipePayload(event)) {
		dataTransfer.dropEffect = 'none'
		return
	}
	event.preventDefault()
	dataTransfer.dropEffect = 'copy'
}

function onDragLeave(event: DragEvent) {
	if (!hasRecipePayload(event)) return
	dragDepth.value = Math.max(0, dragDepth.value - 1)
}

function onDrop(event: DragEvent) {
	dragDepth.value = 0
	const dataTransfer = event.dataTransfer
	if (!dataTransfer || !hasRecipePayload(event)) return
	const raw = dataTransfer.getData(RECIPE_SLOT_MIME_TYPE) || dataTransfer.getData('text/plain')
	const value = parseSlotValue(raw)
	if (!value) return
	event.preventDefault()
	emit('dropValue', value)
}

function onCountUpdate(raw: string) {
	const parsed = Math.round(Number(raw))
	const count = Number.isFinite(parsed) ? Math.min(999, Math.max(1, parsed)) : 1
	emit('updateCount', count)
}
</script>

<template>
	<div
		class="recipe-slot-cell"
		:class="{ 'is-drag-target': dragActive }"
		:data-recipe-slot="slot"
		@axolotl-recipe-slot-drop="onSlotDropEvent"
		@dragenter="onDragEnter"
		@dragover="onDragOver"
		@dragleave="onDragLeave"
		@drop="onDrop"
	>
		<label
			v-if="showCountField && slot === 'stonecutter.result'"
			class="recipe-count-field recipe-count-field-above"
		>
			<span>{{ formatMessage(messages.resultCount) }}</span>
			<StyledInput
				:model-value="String(count)"
				input-attrs="{ type: 'number', min: 1, max: 64 }"
				size="small"
				@update:model-value="onCountUpdate(String($event))"
			/>
		</label>
		<button
			type="button"
			class="recipe-slot-button"
			:class="{ 'recipe-result-button': result }"
			:title="slotLabel"
			:aria-label="slotLabel"
			@click="emit('clear')"
		>
			<RecipeItemIcon :display="display" :atlas="atlas" :size="48" />
		</button>
		<label v-if="showCountField && slot !== 'stonecutter.result'" class="recipe-count-field">
			<span>{{ formatMessage(messages.resultCount) }}</span>
			<StyledInput
				:model-value="String(count)"
				input-attrs="{ type: 'number', min: 1, max: 999 }"
				size="small"
				@update:model-value="onCountUpdate(String($event))"
			/>
		</label>
	</div>
</template>

<style scoped>
.recipe-slot-cell {
	display: flex;
	min-width: 0;
	flex-direction: column;
	align-items: center;
	gap: 0.35rem;
}

.recipe-slot-button {
	display: flex;
	width: 3.5rem;
	height: 3.5rem;
	align-items: center;
	justify-content: center;
	border: 2px solid var(--color-surface-5);
	border-radius: var(--radius-sm);
	background: var(--color-surface-2);
	padding: 0;
	box-shadow:
		inset 1px 1px 0 rgb(0 0 0 / 20%),
		inset -1px -1px 0 rgb(255 255 255 / 10%);
	cursor: pointer;
	transition:
		border-color 0.15s ease,
		background-color 0.15s ease;
}

.recipe-slot-button:hover {
	border-color: var(--color-brand);
	background: var(--color-surface-3);
}

.recipe-slot-button:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 1px;
}

.is-drag-target .recipe-slot-button {
	border-color: var(--color-brand);
	background: var(--color-brand-highlight);
	cursor: copy;
}

.recipe-result-button {
	border-color: color-mix(in srgb, var(--color-brand) 55%, var(--color-surface-5));
}

.recipe-count-field {
	display: flex;
	align-items: center;
	gap: 0.3rem;
	color: #000;
	font-size: 0.65rem;
}

.recipe-count-field-above {
	order: -1;
}

.recipe-count-field :deep(.relative) {
	width: 4.25rem;
}

@media (max-width: 32rem) {
	.recipe-slot-button {
		width: 3.5rem;
		height: 3.5rem;
	}
}
</style>
