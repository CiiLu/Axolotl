<template>
	<div
		data-onboarding-id="creation-import"
		class="flex flex-col gap-6 lg:flex-row lg:items-stretch"
	>
		<!-- ── Left panel: Find a modpack ── -->
		<Card class="flex flex-1 flex-col gap-4 bg-surface-2 !p-6 !rounded-2xl">
			<span class="font-semibold text-contrast">{{
				formatMessage(messages.findModpackPrompt)
			}}</span>

			<Combobox
				v-model="ctx.modpackSearchProjectId.value"
				v-tooltip="ctx.finishDisabled.value ? ctx.finishDisabledTooltip.value : undefined"
				:options="ctx.modpackSearchOptions.value"
				searchable
				:disabled="ctx.finishDisabled.value"
				:search-placeholder="formatMessage(messages.searchModpackPlaceholder)"
				:no-options-message="
					searchLoading
						? formatMessage(commonMessages.loadingLabel)
						: formatMessage(messages.noResultsFound)
				"
				:disable-search-filter="true"
				@search-input="(query: string) => handleSearch(query)"
			>
				<template #option-suffix>
					<RightArrowIcon
						class="size-5 shrink-0 text-secondary opacity-0 transition-opacity group-hover/option:opacity-100 group-data-[focused=true]/option:opacity-100"
					/>
				</template>
			</Combobox>

			<div class="flex items-center gap-3">
				<div class="h-px flex-1 bg-surface-5" />
				<span class="text-sm text-secondary">{{ formatMessage(commonMessages.orLabel) }}</span>
				<div class="h-px flex-1 bg-surface-5" />
			</div>

			<div class="flex gap-2">
				<ButtonStyled type="outlined">
					<button
						v-tooltip="ctx.finishDisabled.value ? ctx.finishDisabledTooltip.value : undefined"
						class="flex-1"
						:disabled="ctx.finishDisabled.value"
						@click="triggerFileInput"
					>
						<ImportIcon />
						{{ formatMessage(messages.importModpack) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						v-tooltip="ctx.finishDisabled.value ? ctx.finishDisabledTooltip.value : undefined"
						class="flex-1"
						:disabled="ctx.finishDisabled.value"
						@click="ctx.browseModpacks()"
					>
						<CompassIcon />
						{{ formatMessage(messages.browseModpacks) }}
					</button>
				</ButtonStyled>
			</div>
		</Card>

		<!-- ── Right panel: Drop zone ── -->
		<div class="flex flex-1 flex-col gap-3">
			<!-- Drop box (fills height) -->
			<div
				class="flex flex-1 flex-col items-center justify-center rounded-2xl border border-dashed border-surface-4 bg-surface-2 p-6"
			>
				<button
					class="flex cursor-pointer flex-col items-center justify-center gap-4 rounded-xl py-4 transition-colors"
					:class="isDragOver ? 'bg-brand-highlight' : 'hover:bg-surface-3'"
					:disabled="ctx.finishDisabled.value"
					@click="triggerFileInput"
					@dragover.prevent="onDragOver"
					@dragleave.prevent="onDragLeave"
					@drop.prevent="onDrop"
				>
					<div
						class="flex size-14 items-center justify-center rounded-full border-2 border-dashed"
						:class="isDragOver ? 'border-brand bg-brand-highlight' : 'border-surface-5'"
					>
						<FolderUpIcon class="size-7" :class="isDragOver ? 'text-brand' : 'text-secondary'" />
					</div>
					<span
						class="text-sm text-center"
						:class="isDragOver ? 'text-brand font-medium' : 'text-secondary'"
					>
						{{ formatMessage(messages.dropZoneClick) }}
					</span>
				</button>
			</div>

			<!-- Launcher icons + caption (below drop box) -->
			<div class="flex flex-col items-center gap-2">
				<div class="flex items-end justify-center">
					<div
						v-for="(item, i) in launcherIcons"
						:key="item.key"
						class="flex size-10 items-center justify-center rounded-xl border border-surface-4 bg-surface-3 shadow-sm"
						:style="{
							transform: `rotate(${(i - 1) * 7}deg) translateY(${Math.abs(i - 1) * -4}px)`,
							marginLeft: i > 0 ? '-4px' : '0',
							zIndex: 3 - Math.abs(i - 1),
						}"
					>
						<img :src="item.url" class="size-5" :alt="item.alt" />
					</div>
				</div>
				<span class="text-center text-sm text-secondary">
					{{ formatMessage(messages.importPrompt) }}
				</span>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { CompassIcon, FolderUpIcon, ImportIcon, RightArrowIcon } from '@modrinth/assets'
import { commonMessages, defineMessages, useVIntl } from '@modrinth/ui'
import { defineAsyncComponent, h, onMounted, ref, watch } from 'vue'

import { useDebugLogger } from '#ui/composables/debug-logger'

import { injectFilePicker } from '../../../../providers'
import ButtonStyled from '../../../base/ButtonStyled.vue'
import Card from '../../../base/Card.vue'
import Combobox from '../../../base/Combobox.vue'
import { injectCreationFlowContext } from '../creation-flow-context'

const debug = useDebugLogger('ImportInstanceStage')
const ctx = injectCreationFlowContext()
const filePicker = injectFilePicker()!
const { formatMessage } = useVIntl()

const searchLoading = ref(false)
const isDragOver = ref(false)

// ── Launcher icons (3, arc arrangement) ──
// @ts-ignore — Vite resolves .ico as static asset URL
import pcl2CeUrl from '@modrinth/assets/icons/PCL2_CE.ico'
// @ts-ignore — Vite resolves .ico as static asset URL
import pcl2Url from '@modrinth/assets/icons/PCL2.ico'
// @ts-ignore — Vite resolves .ico as static asset URL
import hmclUrl from '@modrinth/assets/icons/HMCL.ico'

const launcherIcons = [
	{ key: 'pcl2ce', url: pcl2CeUrl, alt: 'PCL2 CE' },
	{ key: 'pcl2', url: pcl2Url, alt: 'PCL2' },
	{ key: 'hmcl', url: hmclUrl, alt: 'HMCL' },
]

const messages = defineMessages({
	findModpackPrompt: {
		id: 'creation-flow.modal.import-instance.find-modpack.prompt',
		defaultMessage: 'Find a modpack to install',
	},
	searchModpackPlaceholder: {
		id: 'creation-flow.modal.import-instance.search-modpack.placeholder',
		defaultMessage: 'Search for modpack',
	},
	noResultsFound: {
		id: 'creation-flow.modal.import-instance.search-modpack.no-results',
		defaultMessage: 'No results found',
	},
	importModpack: {
		id: 'creation-flow.modal.import-instance.action.import-modpack',
		defaultMessage: 'Import file',
	},
	browseModpacks: {
		id: 'creation-flow.modal.import-instance.action.browse-modpacks',
		defaultMessage: 'Browse modpacks',
	},
	dropZoneClick: {
		id: 'creation-flow.modal.import-instance.drop-zone.click',
		defaultMessage: 'Click to select a file or drag & drop any file/folder',
	},
	importPrompt: {
		id: 'creation-flow.modal.import-instance.import-prompt',
		defaultMessage:
			'Drag & drop launcher folders, modpack files, or .minecraft folders to import an instance in one click',
	},
})

// ── Drop zone handlers ──

function onDragOver() {
	isDragOver.value = true
}

function onDragLeave() {
	isDragOver.value = false
}

function onDrop(event: DragEvent) {
	isDragOver.value = false

	const files = event.dataTransfer?.files
	if (!files || files.length === 0) return

	const file = files[0]
	// Tauri adds a `path` property to File objects from native drag-drop
	const filePath: string | null = (file as any).path ?? null

	if (ctx.onImportFileReceived) {
		ctx.onImportFileReceived({
			file: filePath ? null : file,
			filePath,
			source: 'drag-drop',
		})
		return
	}

	// Fallback: treat as file-picker import
	ctx.modpackFile.value = filePath ? null : file
	ctx.modpackFilePath.value = filePath
	proceedWithModpack()
}

// ── Modpack search logic ──

function proceedWithModpack() {
	if (ctx.finishDisabled.value) return

	debug('proceedWithModpack:', {
		flowType: ctx.flowType,
		modpackSelection: ctx.modpackSelection.value,
	})
	if (ctx.flowType === 'instance') {
		ctx.finish()
	} else {
		ctx.modal.value?.setStage('final-config')
	}
}

const search = async (query: string) => {
	query = query.trim()
	debug('search() called:', { query })

	try {
		const results = await ctx.searchModpacks(query, 10)

		ctx.modpackSearchHits.value = {}
		for (const hit of results.hits) {
			ctx.modpackSearchHits.value[hit.project_id] = {
				title: hit.title,
				iconUrl: hit.icon_url,
				latestVersion: hit.latest_version,
			}
		}

		ctx.modpackSearchOptions.value = results.hits.map((hit) => ({
			label: hit.title,
			value: hit.project_id,
			icon: defineAsyncComponent(() =>
				Promise.resolve({
					setup: () => () =>
						h('img', {
							src: hit.icon_url,
							alt: hit.title,
							class: 'h-5 w-5 rounded',
						}),
				}),
			),
		}))
	} catch (err) {
		debug('search() ERROR:', err)
		ctx.modpackSearchOptions.value = []
	}
	searchLoading.value = false
}

const handleSearch = async (query: string) => {
	debug('handleSearch() called:', { query })
	searchLoading.value = true
	await search(query)
}

onMounted(() => {
	debug('onMounted() firing, resetting and calling search("")')
	ctx.modpackSearchProjectId.value = undefined
	search('')
})

// When a project is selected via search, fetch its latest version and auto-proceed
watch(
	() => ctx.modpackSearchProjectId.value,
	async (projectId, oldProjectId) => {
		if (projectId === oldProjectId) return

		ctx.modpackSearchVersionId.value = undefined
		ctx.modpackVersionOptions.value = []

		if (!projectId) return

		const hit = ctx.modpackSearchHits.value[projectId]

		try {
			const versions = await ctx.getProjectVersions(projectId)
			if (ctx.modpackSearchProjectId.value !== projectId) return
			if (versions.length > 0) {
				const version = versions[0]
				ctx.modpackSelection.value = {
					projectId,
					versionId: version.id,
					name: hit?.title ?? '',
					iconUrl: hit?.iconUrl,
				}
				proceedWithModpack()
			}
		} catch {
			// Failed to fetch versions — do nothing
		}
	},
)

// ── File handling (reserved interface) ──
export interface ImportFilePayload {
	file: File | null
	filePath: string | null
	source: 'file-picker' | 'drag-drop'
}

async function triggerFileInput() {
	if (ctx.finishDisabled.value) return

	// Open a single file dialog — any file type, no extension filter.
	// Folders are handled via drag & drop.
	const picked = (await filePicker.pickFiles?.({ multiple: false })) ?? []

	if (picked.length > 0) {
		const first = picked[0]
		if (ctx.onImportFileReceived) {
			ctx.onImportFileReceived({
				file: first.file ?? null,
				filePath: first.path ?? null,
				source: 'file-picker',
			})
			return
		}
		ctx.modpackFile.value = first.file ?? null
		ctx.modpackFilePath.value = first.path ?? null
		proceedWithModpack()
	}
}
</script>
