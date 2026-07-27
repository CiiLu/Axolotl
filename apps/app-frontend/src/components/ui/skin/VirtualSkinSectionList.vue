<script setup lang="ts">
import { DropdownIcon, EditIcon, PlusIcon, TrashIcon, UnknownIcon, XIcon } from '@modrinth/assets'
import {
	Accordion,
	ButtonStyled,
	commonMessages,
	defineMessages,
	NewModal,
	SkinButton,
	SkinLikeTextButton,
	useScrollViewport,
	useVIntl,
} from '@modrinth/ui'
import { useElementSize, useWindowSize } from '@vueuse/core'
import { Tooltip } from 'floating-vue'
import { computed, nextTick, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import Draggable from 'vuedraggable'

import type { RenderResult } from '@/helpers/rendering/batch-skin-renderer.ts'
import type { Skin } from '@/helpers/skins.ts'

type SkinSectionKind = 'saved' | 'default'
type SkinLikeTextButtonExpose = {
	getRootElement: () => HTMLElement | null | undefined
}
type AddSkinButtonRef = SkinLikeTextButtonExpose | SkinLikeTextButtonExpose[]

interface DefaultSkinSection {
	title: string
	infoTooltip?: string
	skins: Skin[]
}

interface SkinSection {
	key: string
	title: string
	kind: SkinSectionKind
	infoTooltip?: string
	skins: Skin[]
}

interface VirtualSkinSection {
	section: SkinSection
	top: number
	index: number
}

const SKIN_CARD_ASPECT_WIDTH = 31
const SKIN_CARD_ASPECT_HEIGHT = 40
const SKIN_GRID_GAP = 12
const SKIN_SECTION_FIRST_SPACING = 4
const SKIN_SECTION_SPACING = 24
const SKIN_SECTION_HEADER_HEIGHT = 28
const SKIN_SECTION_CONTENT_SPACING = 8
const SAVED_FAVORITES_CONTROL_HEIGHT = 48
const SKIN_SECTION_OVERSCAN = 900
const FALLBACK_CARD_WIDTH = 220
const UNCATEGORIZED_FOLDER_KEY = '__uncategorized__'
const messages = defineMessages({
	savedSkinsSection: {
		id: 'app.skins.section.saved-skins',
		defaultMessage: 'Saved skins',
	},
	addSkinButton: {
		id: 'app.skins.add-button',
		defaultMessage: 'Add skin',
	},
	dragAndDropSubtitle: {
		id: 'app.skins.add-button.drag-and-drop',
		defaultMessage: 'Drag and drop',
	},
	editSkinButton: {
		id: 'app.skins.edit-button',
		defaultMessage: 'Edit skin',
	},
	deleteSkinButton: {
		id: 'app.skins.delete-button',
		defaultMessage: 'Delete skin',
	},
	allFavorites: { id: 'app.skins.favorites.all', defaultMessage: 'All skins' },
	uncategorized: { id: 'app.skins.favorites.uncategorized', defaultMessage: 'Uncategorized' },
	createFavoriteButton: {
		id: 'app.skins.favorites.create-button',
		defaultMessage: 'Create favorite folder',
	},
	createFavoritePlaceholder: {
		id: 'app.skins.favorites.create-placeholder',
		defaultMessage: 'Folder name',
	},
	createFavoriteTitle: {
		id: 'app.skins.favorites.create-title',
		defaultMessage: 'Create favorite folder',
	},
	createFavoriteConfirm: {
		id: 'app.skins.favorites.create-confirm',
		defaultMessage: 'Create folder',
	},
	emptyFavorite: {
		id: 'app.skins.favorites.empty',
		defaultMessage: 'No saved skins are in this favorite folder yet.',
	},
	nameRequired: {
		id: 'app.skins.favorites.error-name-required',
		defaultMessage: 'Enter a folder name.',
	},
	duplicateName: {
		id: 'app.skins.favorites.error-duplicate-name',
		defaultMessage: 'A favorite folder with this name already exists.',
	},
})

const props = defineProps<{
	savedSkins: Skin[]
	defaultSkinSections: DefaultSkinSection[]
	getBakedSkinTextures: (skin: Skin) => RenderResult | undefined
	isSkinSelected: (skin: Skin) => boolean
	isSkinActive: (skin: Skin) => boolean
	isAddSkinButtonDragActive: boolean
	readOnly?: boolean
	activeTab?: 'saved' | 'default'
	favoriteFolders: string[]
	selectedFavoriteFolder: string | null
	favoriteAssignments: Record<string, string>
}>()

const emit = defineEmits<{
	select: [skin: Skin]
	edit: [skin: Skin, event: MouseEvent]
	delete: [skin: Skin]
	'reorder-saved-skins': [skins: Skin[]]
	'create-favorite-folder': [name: string]
	'select-favorite-folder': [name: string | null]
	'delete-favorite-folder': [name: string]
	'add-skin': []
	'add-skin-dragenter': [event: DragEvent]
	'add-skin-dragover': [event: DragEvent]
	'add-skin-dragleave': [event: DragEvent]
	'add-skin-drop': [event: DragEvent]
}>()

const addSkinButton = useTemplateRef<AddSkinButtonRef>('addSkinButton')
const { formatMessage } = useVIntl()
const { listContainer, relativeScrollTop, scrollContainer, viewportHeight } = useScrollViewport()
const openSectionKeys = ref<Set<string>>(new Set())
const hasSettledInitialLayout = ref(false)
const knownSectionKeys = new Set<string>()
let enableLayoutTransitionsFrame: number | null = null
let isEnableLayoutTransitionsScheduled = false
let isUnmounted = false

const { width: listWidth } = useElementSize(listContainer)
const { width: windowWidth } = useWindowSize()

const columnCount = computed(() => {
	if (windowWidth.value >= 2050) {
		return 6
	}

	if (windowWidth.value >= 1750) {
		return 5
	}

	if (windowWidth.value >= 1300) {
		return 4
	}

	return 3
})

const cardWidth = computed(() => {
	if (listWidth.value <= 0) {
		return FALLBACK_CARD_WIDTH
	}

	const gapsWidth = (columnCount.value - 1) * SKIN_GRID_GAP
	return Math.max(0, (listWidth.value - gapsWidth) / columnCount.value)
})

const cardHeight = computed(
	() => (cardWidth.value * SKIN_CARD_ASPECT_HEIGHT) / SKIN_CARD_ASPECT_WIDTH,
)

const sections = computed<SkinSection[]>(() => {
	if (props.activeTab === 'saved') {
		return [
			{
				key: 'saved-skins',
				title: formatMessage(messages.savedSkinsSection),
				kind: 'saved',
				skins: filteredSavedSkins.value,
			},
		]
	}

	if (props.activeTab === 'default') {
		return props.defaultSkinSections.map((section) => ({
			key: defaultSkinSectionKey(section.title),
			title: section.title,
			kind: 'default' as const,
			infoTooltip: section.infoTooltip,
			skins: section.skins,
		}))
	}

	return [
		{
			key: 'saved-skins',
			title: formatMessage(messages.savedSkinsSection),
			kind: 'saved',
			skins: filteredSavedSkins.value,
		},
		...props.defaultSkinSections.map((section) => ({
			key: defaultSkinSectionKey(section.title),
			title: section.title,
			kind: 'default' as const,
			infoTooltip: section.infoTooltip,
			skins: section.skins,
		})),
	]
})
const draggableSavedSkins = ref<Skin[]>([])
const isDraggingSavedSkin = ref(false)
const draggedSavedSkin = ref<Skin | null>(null)
const isFavoriteMenuOpen = ref(false)
const createFavoriteModal = ref<InstanceType<typeof NewModal>>()
const newFavoriteName = ref('')
const favoriteError = ref('')
const filteredSavedSkins = computed(() => {
	if (!props.selectedFavoriteFolder) return props.savedSkins

	if (props.selectedFavoriteFolder === UNCATEGORIZED_FOLDER_KEY) {
		return props.savedSkins.filter(
			(skin) => !props.favoriteAssignments[savedSkinKey(skin)],
		)
	}

	return props.savedSkins.filter(
		(skin) => props.favoriteAssignments[savedSkinKey(skin)] === props.selectedFavoriteFolder,
	)
})
const selectedFavoriteLabel = computed(() => {
	if (props.selectedFavoriteFolder === null) return formatMessage(messages.allFavorites)
	if (props.selectedFavoriteFolder === UNCATEGORIZED_FOLDER_KEY) return formatMessage(messages.uncategorized)
	return props.selectedFavoriteFolder
})
const canReorderSavedSkins = computed(() => draggableSavedSkins.value.length > 1)
const fixedSavedSkins = computed(() =>
	filteredSavedSkins.value.filter((skin) => !canDragSavedSkin(skin)),
)

const sectionLayouts = computed(() => {
	const layouts: Array<{ section: SkinSection; top: number; height: number; index: number }> = []
	let top = 0

	sections.value.forEach((section, index) => {
		const height = getSectionHeightEstimate(section, index)
		layouts.push({ section, top, height, index })
		top += height
	})

	return layouts
})

const totalHeight = computed(() => {
	const lastSection = sectionLayouts.value[sectionLayouts.value.length - 1]
	return lastSection ? lastSection.top + lastSection.height : 0
})

const visibleSections = computed<VirtualSkinSection[]>(() => {
	if (!listContainer.value || !scrollContainer.value) {
		return sectionLayouts.value.slice(0, 4)
	}

	const viewportStart = Math.max(0, relativeScrollTop.value - SKIN_SECTION_OVERSCAN)
	const viewportEnd = relativeScrollTop.value + viewportHeight.value + SKIN_SECTION_OVERSCAN

	return sectionLayouts.value
		.filter((layout) => layout.top + layout.height >= viewportStart && layout.top <= viewportEnd)
		.map(({ section, top, index }) => ({ section, top, index }))
})

watch(
	sections,
	(nextSections) => {
		const sectionKeys = new Set(nextSections.map((section) => section.key))
		const openKeys = new Set(openSectionKeys.value)

		for (const section of nextSections) {
			if (!knownSectionKeys.has(section.key)) {
				knownSectionKeys.add(section.key)
				openKeys.add(section.key)
			}
		}

		for (const key of knownSectionKeys) {
			if (!sectionKeys.has(key)) {
				knownSectionKeys.delete(key)
				openKeys.delete(key)
			}
		}

		openSectionKeys.value = openKeys
	},
	{ immediate: true },
)

watch(
	() => filteredSavedSkins.value,
	(nextSkins) => {
		if (isDraggingSavedSkin.value) {
			return
		}

		draggableSavedSkins.value = nextSkins.filter(canDragSavedSkin)
	},
	{ immediate: true },
)

watch(
	listWidth,
	(width) => {
		if (
			typeof window === 'undefined' ||
			width <= 0 ||
			hasSettledInitialLayout.value ||
			isEnableLayoutTransitionsScheduled
		) {
			return
		}

		isEnableLayoutTransitionsScheduled = true
		void nextTick(() => {
			if (isUnmounted) return

			enableLayoutTransitionsFrame = window.requestAnimationFrame(() => {
				if (isUnmounted) return

				enableLayoutTransitionsFrame = window.requestAnimationFrame(() => {
					if (isUnmounted) return

					hasSettledInitialLayout.value = true
					enableLayoutTransitionsFrame = null
					isEnableLayoutTransitionsScheduled = false
				})
			})
		})
	},
	{ immediate: true },
)

onUnmounted(() => {
	isUnmounted = true

	if (enableLayoutTransitionsFrame !== null) {
		window.cancelAnimationFrame(enableLayoutTransitionsFrame)
	}
})

function defaultSkinSectionKey(title: string) {
	return `default-skins-${title}`
}

function skinKey(skin: Skin, prefix: string) {
	return `${prefix}-${skin.source}-${skin.texture_key}-${skin.variant}-${skin.cape_id ?? 'no-cape'}`
}

function savedSkinKey(skin: Skin) {
	return skinKey(skin, 'saved-skin')
}

function canDragSavedSkin(skin: Skin) {
	return skin.source === 'custom' || skin.source === 'custom_external'
}

function doSkinOrdersMatch(firstSkins: Skin[], secondSkins: Skin[]) {
	const draggableSecondSkins = secondSkins.filter(canDragSavedSkin)

	return (
		firstSkins.length === draggableSecondSkins.length &&
		firstSkins.every(
			(skin, index) => savedSkinKey(skin) === savedSkinKey(draggableSecondSkins[index]),
		)
	)
}

function onSavedSkinDragStart(event: { oldIndex?: number }) {
	isDraggingSavedSkin.value = true
	draggedSavedSkin.value = draggableSavedSkins.value[event.oldIndex ?? -1] ?? null
}

function onSavedSkinDragEnd() {
	isDraggingSavedSkin.value = false
	draggedSavedSkin.value = null

	if (doSkinOrdersMatch(draggableSavedSkins.value, filteredSavedSkins.value)) {
		draggableSavedSkins.value = filteredSavedSkins.value.filter(canDragSavedSkin)
		return
	}

	emit('reorder-saved-skins', [...draggableSavedSkins.value])
}

function toggleFavoriteMenu() {
	isFavoriteMenuOpen.value = !isFavoriteMenuOpen.value
}

function selectFavoriteFolder(name: string | null) {
	emit('select-favorite-folder', name)
	isFavoriteMenuOpen.value = false
}

function openCreateFavoriteModal() {
	newFavoriteName.value = ''
	favoriteError.value = ''
	createFavoriteModal.value?.show()
}

function closeCreateFavoriteModal() {
	createFavoriteModal.value?.hide()
}

function createFavoriteFolder() {
	const name = newFavoriteName.value.trim()
	if (!name) {
		favoriteError.value = formatMessage(messages.nameRequired)
		return
	}
	if (
		props.favoriteFolders.some(
			(folder) => folder.localeCompare(name, undefined, { sensitivity: 'accent' }) === 0,
		)
	) {
		favoriteError.value = formatMessage(messages.duplicateName)
		return
	}
	favoriteError.value = ''
	newFavoriteName.value = ''
	emit('create-favorite-folder', name)
	createFavoriteModal.value?.hide()
}

function deleteFavoriteFolder(name: string) {
	emit('delete-favorite-folder', name)
	isFavoriteMenuOpen.value = false
}

function isSectionOpen(key: string) {
	return openSectionKeys.value.has(key)
}

function setSectionOpen(key: string, open: boolean) {
	const openKeys = new Set(openSectionKeys.value)

	if (open) {
		openKeys.add(key)
	} else {
		openKeys.delete(key)
	}

	openSectionKeys.value = openKeys
}

function getSectionHeightEstimate(section: SkinSection, index: number) {
	const spacing = index === 0 ? SKIN_SECTION_FIRST_SPACING : SKIN_SECTION_SPACING

	if (!isSectionOpen(section.key)) {
		return spacing + SKIN_SECTION_HEADER_HEIGHT
	}

	const cardCount = section.kind === 'saved' ? section.skins.length + 1 : section.skins.length
	const rowCount = Math.ceil(cardCount / columnCount.value)
	const gridHeight = rowCount * cardHeight.value + Math.max(0, rowCount - 1) * SKIN_GRID_GAP

	const controlsHeight = section.kind === 'saved' ? SAVED_FAVORITES_CONTROL_HEIGHT : 0

	return (
		spacing +
		SKIN_SECTION_HEADER_HEIGHT +
		SKIN_SECTION_CONTENT_SPACING +
		controlsHeight +
		gridHeight
	)
}

function getAddSkinButtonElement() {
	const button = Array.isArray(addSkinButton.value)
		? addSkinButton.value.find((candidate) => candidate.getRootElement())
		: addSkinButton.value

	return button?.getRootElement()
}

defineExpose({ getAddSkinButtonElement })
</script>

<template>
	<NewModal
		ref="createFavoriteModal"
		:header="formatMessage(messages.createFavoriteTitle)"
		max-width="420px"
	>
		<form class="flex flex-col gap-4" @submit.prevent="createFavoriteFolder">
			<input
				v-model="newFavoriteName"
				type="text"
				class="h-10 rounded-xl border border-solid border-button-border bg-bg-raised px-3 text-sm text-primary"
				:placeholder="formatMessage(messages.createFavoritePlaceholder)"
				autofocus
				@input="favoriteError = ''"
			/>
			<p v-if="favoriteError" class="m-0 text-xs font-semibold text-red">
				{{ favoriteError }}
			</p>
		</form>
		<template #actions>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button type="button" @click="closeCreateFavoriteModal">
						<XIcon /> {{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" @click="createFavoriteFolder">
						<PlusIcon /> {{ formatMessage(messages.createFavoriteConfirm) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>

	<div
		ref="listContainer"
		class="relative w-full"
		:style="{ height: `${totalHeight}px`, overflowAnchor: 'none' }"
	>
		<div
			v-for="{ section, top, index } in visibleSections"
			:key="section.key"
			class="absolute inset-x-0"
			:class="[
				index === 0 ? 'pt-1' : 'pt-6',
				hasSettledInitialLayout
					? 'transition-transform duration-300 ease-in-out will-change-transform motion-reduce:transition-none'
					: '',
			]"
			:style="{ transform: `translateY(${top}px)` }"
		>
			<Accordion
				button-class="group flex w-full items-center gap-[6px] bg-transparent m-0 p-0 border-none cursor-pointer text-left"
				content-class="pt-2"
				:open-by-default="isSectionOpen(section.key)"
				@on-open="setSectionOpen(section.key, true)"
				@on-close="setSectionOpen(section.key, false)"
			>
				<template #title>
					{{ section.title }}
				</template>
				<template #button="{ open }">
					<DropdownIcon
						class="size-6 shrink-0 text-primary transition-transform duration-300"
						:class="{ 'rotate-180': open }"
					/>
					<span class="min-w-0 text-xl font-semibold leading-7 text-primary">
						{{ section.title }}
					</span>
					<Tooltip
						v-if="section.infoTooltip"
						theme="dismissable-prompt"
						placement="top"
						:triggers="['hover', 'focus']"
					>
						<span
							class="inline-flex size-6 shrink-0 items-center justify-center text-secondary transition-colors group-hover:text-primary"
							@click.stop
						>
							<UnknownIcon class="size-5" />
						</span>
						<template #popper>
							<p class="m-0 max-w-96 text-wrap text-sm font-medium leading-tight">
								{{ section.infoTooltip }}
							</p>
						</template>
					</Tooltip>
				</template>

				<div v-if="section.kind === 'saved'" class="mb-3 flex flex-wrap items-start gap-2">
					<div class="relative min-w-64">
						<button
							type="button"
							class="flex h-10 w-full items-center justify-between gap-2 rounded-xl bg-button-bg px-4 text-left text-sm font-semibold text-primary shadow-[var(--shadow-inset-sm)] transition-colors hover:brightness-95"
							:class="{ 'rounded-b-none': isFavoriteMenuOpen }"
							:aria-expanded="isFavoriteMenuOpen"
							@click="toggleFavoriteMenu"
						>
							<span class="min-w-0 truncate">{{ selectedFavoriteLabel }}</span>
							<DropdownIcon
								class="size-5 shrink-0 transition-transform duration-200"
								:class="{ 'rotate-180': isFavoriteMenuOpen }"
							/>
						</button>
						<div
							v-show="isFavoriteMenuOpen"
							class="absolute z-20 max-h-80 w-full overflow-y-auto rounded-b-xl bg-button-bg shadow-[var(--shadow-inset-sm)]"
						>
							<button
								type="button"
								class="flex w-full items-center px-4 py-3 text-left text-sm font-semibold text-primary transition-colors hover:brightness-90"
								:class="{ 'bg-brand text-accent-contrast': selectedFavoriteFolder === null }"
								@click="selectFavoriteFolder(null)"
							>
								{{ formatMessage(messages.allFavorites) }}
							</button>
							<button
								type="button"
								class="flex w-full items-center px-4 py-3 text-left text-sm font-semibold text-primary transition-colors hover:brightness-90"
								:class="{ 'bg-brand text-accent-contrast': selectedFavoriteFolder === UNCATEGORIZED_FOLDER_KEY }"
								@click="selectFavoriteFolder(UNCATEGORIZED_FOLDER_KEY)"
							>
								{{ formatMessage(messages.uncategorized) }}
							</button>
							<div
								v-for="folder in favoriteFolders"
								:key="folder"
								:data-skin-favorite-drop-target="folder"
								class="group flex items-center transition-colors hover:brightness-90"
								:class="{ 'bg-brand text-accent-contrast': selectedFavoriteFolder === folder }"
							>
								<button
									type="button"
									class="min-w-0 flex-1 bg-transparent px-4 py-3 text-left text-sm font-semibold text-inherit"
									@click="selectFavoriteFolder(folder)"
								>
									<span class="block truncate">{{ folder }}</span>
								</button>
								<button
									v-if="!readOnly"
									type="button"
									class="mr-2 flex size-8 items-center justify-center rounded-lg bg-transparent text-inherit opacity-70 transition-opacity hover:opacity-100"
									:aria-label="formatMessage(messages.deleteSkinButton)"
									@click.stop="deleteFavoriteFolder(folder)"
								>
									<TrashIcon class="size-4" />
								</button>
							</div>
						</div>
					</div>
					<ButtonStyled v-if="!readOnly" circular>
						<button
							type="button"
							:aria-label="formatMessage(messages.createFavoriteButton)"
							@click="openCreateFavoriteModal"
						>
							<PlusIcon />
						</button>
					</ButtonStyled>
				</div>
				<p
					v-if="section.kind === 'saved' && section.skins.length === 0"
					class="m-0 rounded-xl bg-bg-raised p-4 text-sm font-semibold text-secondary"
				>
					{{ formatMessage(messages.emptyFavorite) }}
				</p>

				<Draggable
					v-if="section.kind === 'saved'"
					:list="draggableSavedSkins"
					class="grid w-full grid-cols-3 gap-3 min-[1300px]:grid-cols-4 min-[1750px]:grid-cols-5 min-[2050px]:grid-cols-6"
					:item-key="savedSkinKey"
					:disabled="readOnly"
					:animation="250"
					:swap-threshold="1"
					:invert-swap="false"
					:force-fallback="true"
					:fallback-on-body="true"
					:fallback-tolerance="4"
					ghost-class="skin-reorder-ghost"
					chosen-class="skin-reorder-chosen"
					drag-class="skin-reorder-drag"
					fallback-class="skin-reorder-fallback"
					@start="onSavedSkinDragStart"
					@end="onSavedSkinDragEnd"
				>
					<template #header>
						<SkinLikeTextButton
							ref="addSkinButton"
							class="aspect-[31/40] w-full min-w-0 box-border rounded-[20px]"
							dropzone
							:disabled="readOnly"
							:drag-active="!readOnly && isAddSkinButtonDragActive"
							@click="emit('add-skin')"
							@dragenter="emit('add-skin-dragenter', $event)"
							@dragover="emit('add-skin-dragover', $event)"
							@dragleave="emit('add-skin-dragleave', $event)"
							@drop="emit('add-skin-drop', $event)"
						>
							<template #icon>
								<PlusIcon class="size-8" />
							</template>
							{{ formatMessage(messages.addSkinButton) }}
							<template #subtitle>{{ formatMessage(messages.dragAndDropSubtitle) }}</template>
						</SkinLikeTextButton>
					</template>

					<template #item="{ element: skin }">
						<div
							:key="savedSkinKey(skin)"
							class="relative aspect-[31/40] w-full min-w-0 box-border rounded-[20px]"
						>
							<SkinButton
								class="h-full w-full min-w-0 box-border rounded-[20px]"
								:forward-image-src="getBakedSkinTextures(skin)?.forwards"
								:selected="isSkinSelected(skin)"
								:active="isSkinActive(skin)"
								:disabled="readOnly"
								:is-dragging="isDraggingSavedSkin"
								@select="emit('select', skin)"
							>
								<template v-if="!readOnly" #overlay-buttons>
									<ButtonStyled color="brand">
										<button
											:aria-label="formatMessage(messages.editSkinButton)"
											class="pointer-events-auto"
											@click.stop="(event: MouseEvent) => emit('edit', skin, event)"
										>
											<EditIcon /> {{ formatMessage(commonMessages.editButton) }}
										</button>
									</ButtonStyled>
									<ButtonStyled v-show="!skin.is_equipped" circular color="red">
										<button
											v-tooltip="formatMessage(messages.deleteSkinButton)"
											:aria-label="formatMessage(messages.deleteSkinButton)"
											class="!rounded-[100%] pointer-events-auto"
											@click.stop="emit('delete', skin)"
										>
											<TrashIcon />
										</button>
									</ButtonStyled>
								</template>
							</SkinButton>
						</div>
					</template>

					<template #footer>
						<div
							v-for="skin in fixedSavedSkins"
							:key="savedSkinKey(skin)"
							class="relative aspect-[31/40] w-full min-w-0 box-border rounded-[20px]"
						>
							<SkinButton
								class="h-full w-full min-w-0 box-border rounded-[20px]"
								:forward-image-src="getBakedSkinTextures(skin)?.forwards"
								:selected="isSkinSelected(skin)"
								:active="isSkinActive(skin)"
								:disabled="readOnly"
								:is-dragging="isDraggingSavedSkin"
								@select="emit('select', skin)"
							>
								<template v-if="!readOnly" #overlay-buttons>
									<ButtonStyled color="brand">
										<button
											:aria-label="formatMessage(messages.editSkinButton)"
											class="pointer-events-auto"
											@click.stop="(event: MouseEvent) => emit('edit', skin, event)"
										>
											<EditIcon /> {{ formatMessage(commonMessages.editButton) }}
										</button>
									</ButtonStyled>
									<ButtonStyled v-show="!skin.is_equipped" circular color="red">
										<button
											v-tooltip="formatMessage(messages.deleteSkinButton)"
											:aria-label="formatMessage(messages.deleteSkinButton)"
											class="!rounded-[100%] pointer-events-auto"
											@click.stop="emit('delete', skin)"
										>
											<TrashIcon />
										</button>
									</ButtonStyled>
								</template>
							</SkinButton>
						</div>
					</template>
				</Draggable>

				<div
					v-else
					class="grid w-full grid-cols-3 gap-3 min-[1300px]:grid-cols-4 min-[1750px]:grid-cols-5 min-[2050px]:grid-cols-6"
				>
					<SkinButton
						v-for="skin in section.skins"
						:key="skinKey(skin, section.key)"
						class="aspect-[31/40] w-full min-w-0 box-border rounded-[20px]"
						:forward-image-src="getBakedSkinTextures(skin)?.forwards"
						:selected="isSkinSelected(skin)"
						:active="isSkinActive(skin)"
						:tooltip="skin.name"
						:disabled="readOnly"
						:is-dragging="isDraggingSavedSkin"
						@select="emit('select', skin)"
					>
						<template #overlay-buttons>
							<ButtonStyled color="brand">
								<button
									:aria-label="formatMessage(messages.editSkinButton)"
									class="pointer-events-auto"
									@click.stop="(event: MouseEvent) => emit('edit', skin, event)"
								>
									<EditIcon /> {{ formatMessage(commonMessages.editButton) }}
								</button>
							</ButtonStyled>
						</template>
					</SkinButton>
				</div>
			</Accordion>
		</div>
	</div>
</template>

<style scoped>
:global(.skin-reorder-ghost) {
	opacity: 0.35;
}

:global(.skin-reorder-drag) {
	cursor: grabbing;
}

:global(.skin-reorder-fallback) {
	opacity: 0.9;
	pointer-events: none;
}
</style>
