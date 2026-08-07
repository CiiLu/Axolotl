<script setup lang="ts">
import { defineMessages, StyledInput, useVIntl } from '@modrinth/ui'
import Fuse from 'fuse.js'
import { computed, ref, watch } from 'vue'

import type { SlotDisplay } from '@/lab/recipe-generator/display'
import type { TextureAtlas } from '@/lab/recipe-generator/resources'
import type { SlotValue } from '@/lab/recipe-generator/types'

import RecipeItemIcon from './RecipeItemIcon.vue'
import RecipeSlotDragLayer from './RecipeSlotDragLayer.vue'

export type PaletteEntry = {
	key: string
	name: string
	id: string
	display: SlotDisplay
	value: SlotValue
}

const props = defineProps<{
	entries: PaletteEntry[]
	atlas: TextureAtlas
	loading: boolean
}>()

const emit = defineEmits<{
	pick: [value: SlotValue]
}>()

const { formatMessage } = useVIntl()
const search = ref('')
const debouncedSearch = ref('')
let searchTimer: ReturnType<typeof window.setTimeout> | undefined
let lastPickKey = ''
let lastPickAt = 0
const draggingKey = ref('')
let suppressClicksUntil = 0

const RECIPE_SLOT_MIME_TYPE = 'application/x-axolotl-recipe-slot'
const isTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

type StartDrag = (
	event: PointerEvent,
	value: SlotValue,
	display: SlotDisplay,
	atlas: TextureAtlas,
	onFinish?: (moved: boolean) => void,
) => void

const messages = defineMessages({
	searchPlaceholder: {
		id: 'app.lab.recipe-generator.items.search-placeholder',
		defaultMessage: 'Search items',
	},
	loading: { id: 'app.lab.recipe-generator.items.loading', defaultMessage: 'Loading items' },
	empty: {
		id: 'app.lab.recipe-generator.items.empty',
		defaultMessage: 'No items match your search.',
	},
	addItem: { id: 'app.lab.recipe-generator.items.add', defaultMessage: 'Add to recipe' },
})

watch(search, (value) => {
	if (searchTimer) window.clearTimeout(searchTimer)
	searchTimer = window.setTimeout(() => {
		debouncedSearch.value = value
	}, 120)
})

const fuse = computed(
	() =>
		new Fuse(props.entries, {
			keys: ['name', 'id'],
			threshold: 0.35,
			ignoreLocation: true,
		}),
)

const visibleEntries = computed(() => {
	const query = debouncedSearch.value.trim()
	if (!query) return props.entries
	return fuse.value.search(query).map((result) => result.item)
})

function pick(value: SlotValue) {
	const key = JSON.stringify(value)
	const now = Date.now()
	if (key === lastPickKey && now - lastPickAt < 300) return
	lastPickKey = key
	lastPickAt = now
	emit('pick', value)
}

function pickFromClick(event: MouseEvent, value: SlotValue) {
	if (event.detail > 1 || Date.now() < suppressClicksUntil) return
	pick(value)
}

function onDragStart(event: DragEvent, entry: PaletteEntry) {
	if (isTauriRuntime) {
		event.preventDefault()
		return
	}
	const dataTransfer = event.dataTransfer
	if (!dataTransfer) return
	const payload = JSON.stringify(entry.value)
	draggingKey.value = entry.key
	dataTransfer.effectAllowed = 'copy'
	dataTransfer.setData(RECIPE_SLOT_MIME_TYPE, payload)
	dataTransfer.setData('text/plain', payload)
}

function onDragEnd() {
	draggingKey.value = ''
	suppressClicksUntil = Date.now() + 350
}

function startPointerDrag(event: PointerEvent, entry: PaletteEntry, startDrag: StartDrag) {
	if (!isTauriRuntime || event.button !== 0) return
	draggingKey.value = entry.key
	startDrag(event, entry.value, entry.display, props.atlas, (moved) => {
		draggingKey.value = ''
		if (moved) suppressClicksUntil = Date.now() + 350
	})
}
</script>

<template>
	<RecipeSlotDragLayer v-slot="{ startDrag }">
		<div class="flex min-h-0 flex-1 flex-col gap-2">
			<StyledInput
				v-model="search"
				:placeholder="formatMessage(messages.searchPlaceholder)"
				clearable
				class="shrink-0"
			/>
			<div v-if="loading" class="flex min-h-24 items-center justify-center text-sm text-secondary">
				{{ formatMessage(messages.loading) }}
			</div>
			<div
				v-else-if="!visibleEntries.length"
				class="flex min-h-24 items-center justify-center px-4 text-sm text-secondary"
			>
				{{ formatMessage(messages.empty) }}
			</div>
			<div v-else class="recipe-palette-grid">
				<button
					v-for="entry in visibleEntries"
					:key="entry.key"
					type="button"
					:draggable="!isTauriRuntime"
					class="recipe-palette-item"
					:class="{ 'is-dragging': draggingKey === entry.key }"
					:style="{ touchAction: isTauriRuntime ? 'none' : undefined }"
					:title="`${formatMessage(messages.addItem)}: ${entry.name}`"
					:aria-label="`${formatMessage(messages.addItem)}: ${entry.name}`"
					@click="pickFromClick($event, entry.value)"
					@pointerdown="startPointerDrag($event, entry, startDrag)"
					@dragstart="onDragStart($event, entry)"
					@dragend="onDragEnd"
				>
					<RecipeItemIcon :display="entry.display" :atlas="atlas" :size="34" :show-count="false" />
					<span class="recipe-palette-name">{{ entry.name }}</span>
				</button>
			</div>
		</div>
	</RecipeSlotDragLayer>
</template>

<style scoped>
.recipe-palette-grid {
	display: grid;
	min-height: 0;
	flex: 1;
	grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
	grid-auto-rows: 4rem;
	align-content: start;
	gap: 0.4rem;
	overflow-y: auto;
	overscroll-behavior: contain;
	padding: 0.1rem 0.25rem 0.25rem 0.1rem;
}

.recipe-palette-item {
	display: flex;
	min-width: 0;
	align-items: center;
	justify-content: center;
	flex-direction: column;
	gap: 0.2rem;
	overflow: hidden;
	border: 1px solid var(--color-surface-5);
	border-radius: var(--radius-sm);
	background: var(--color-surface-4);
	padding: 0.2rem 0.1rem;
	color: var(--color-contrast);
	cursor: grab;
	transition:
		background-color 0.15s ease,
		border-color 0.15s ease,
		transform 0.1s ease;
}

.recipe-palette-item:hover,
.recipe-palette-item:focus-visible {
	border-color: var(--color-brand);
	background: var(--color-surface-3);
	outline: none;
}

.recipe-palette-item:active {
	cursor: grabbing;
	transform: translateY(1px);
}

.recipe-palette-item.is-dragging {
	opacity: 0.6;
	border-color: var(--color-brand);
}

.recipe-palette-name {
	width: 100%;
	overflow: hidden;
	color: var(--color-secondary);
	font-size: 0.55rem;
	line-height: 1.15;
	text-align: center;
	text-overflow: ellipsis;
	white-space: nowrap;
}
</style>
