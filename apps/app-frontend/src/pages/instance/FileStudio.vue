<script setup lang="ts">
import {
	ChevronDownIcon,
	ChevronRightIcon,
	CodeIcon,
	FileCodeIcon,
	FolderIcon,
	SaveIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { readDir, readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRouter } from 'vue-router'

import StudioEditor from '@/components/instance/studio/StudioEditor.vue'
import StudioEditorTabs from '@/components/instance/studio/StudioEditorTabs.vue'
import {
	type StudioDocument,
	useStudioDocuments,
} from '@/components/instance/studio/useStudioDocuments'
import { get_full_path } from '@/helpers/instance'
import {
	listenStudioFilesChanged,
	registerStudioWatcher,
	unregisterStudioWatcher,
} from '@/helpers/studio'
import type { GameInstance } from '@/helpers/types'

interface StudioTreeNode {
	name: string
	path: string
	type: 'directory' | 'file'
	depth: number
	expanded: boolean
	loaded: boolean
	loading: boolean
	children: StudioTreeNode[]
}

const props = defineProps<{
	instance: GameInstance
}>()

const messages = defineMessages({
	title: {
		id: 'instance.files.studio.title',
		defaultMessage: 'Studio',
	},
	files: {
		id: 'instance.files.studio.files',
		defaultMessage: 'Explorer',
	},
	backToFiles: {
		id: 'instance.files.studio.back-to-files',
		defaultMessage: 'Back to files',
	},
	emptyTitle: {
		id: 'instance.files.studio.empty-title',
		defaultMessage: 'Select a configuration file',
	},
	emptyDescription: {
		id: 'instance.files.studio.empty-description',
		defaultMessage: 'Open a properties, TOML, YAML, or JSON file from the explorer.',
	},
	loadDirectoryFailed: {
		id: 'instance.files.studio.load-directory-failed',
		defaultMessage: 'Could not load directory',
	},
	loadFileFailed: {
		id: 'instance.files.studio.load-file-failed',
		defaultMessage: 'Could not open file',
	},
	saveFailed: {
		id: 'instance.files.studio.save-failed',
		defaultMessage: 'Could not save file',
	},
	discardChanges: {
		id: 'instance.files.studio.discard-changes',
		defaultMessage: 'Discard changes',
	},
	loadingFile: {
		id: 'instance.files.studio.loading-file',
		defaultMessage: 'Loading file...',
	},
})

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const router = useRouter()
const instanceRoot = ref('')
const rootNodes = ref<StudioTreeNode[]>([])
const treeLoading = ref(true)
const fileLoading = ref(false)
const treeScrollElement = ref<HTMLElement | null>(null)
let watcherRegistrationId: string | null = null
let watcherInstanceId: string | null = null
let unlistenStudioFiles: (() => void) | null = null
let watcherGeneration = 0
let changedPaths = new Set<string>()
let changeTimer: ReturnType<typeof setTimeout> | null = null

const {
	documents,
	activeDocument,
	activePath,
	hasUnsavedChanges,
	hasAnyUnsavedChanges,
	activate: activateDocument,
	open: openDocument,
	close: closeDocument,
	saveActive: saveActiveFile,
	saveAll: saveAllDocuments,
	updateActiveContent,
	discardActiveChanges: discardChanges,
	reset: resetDocuments,
} = useStudioDocuments(
	(document, content) => writeTextFile(resolvePath(document.path), content),
	(error) => {
		addNotification({
			title: formatMessage(messages.saveFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	},
)
const selectedName = computed(() => activeDocument.value?.name ?? '')
const editorLanguage = computed(() => {
	const extension = selectedName.value.split('.').pop()?.toLowerCase()
	if (extension === 'yml') return 'yaml'
	return extension ?? 'plaintext'
})
const breadcrumbSegments = computed(() => activePath.value.split('/').filter(Boolean))

function resolvePath(relativePath: string): string {
	return relativePath ? `${instanceRoot.value}/${relativePath}` : instanceRoot.value
}

function isSupportedFile(name: string): boolean {
	return /\.(properties|toml|ya?ml|json)$/i.test(name)
}

async function listDirectory(path: string, depth: number): Promise<StudioTreeNode[]> {
	const entries = await readDir(resolvePath(path))
	return entries
		.map((entry) => ({
			name: entry.name,
			path: path ? `${path}/${entry.name}` : entry.name,
			type: entry.isDirectory ? ('directory' as const) : ('file' as const),
			depth,
			expanded: false,
			loaded: false,
			loading: false,
			children: [],
		}))
		.sort((a, b) => {
			if (a.type !== b.type) return a.type === 'directory' ? -1 : 1
			return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })
		})
}

function collectDirectoryState(
	nodes: StudioTreeNode[],
	state = new Map<string, Pick<StudioTreeNode, 'expanded' | 'loaded'>>(),
) {
	for (const node of nodes) {
		if (node.type !== 'directory') continue
		state.set(node.path, { expanded: node.expanded, loaded: node.loaded })
		collectDirectoryState(node.children, state)
	}
	return state
}

async function restoreLoadedDirectories(
	nodes: StudioTreeNode[],
	state: Map<string, Pick<StudioTreeNode, 'expanded' | 'loaded'>>,
) {
	await Promise.all(
		nodes.map(async (node) => {
			if (node.type !== 'directory') return
			const previous = state.get(node.path)
			if (!previous?.loaded) return
			node.children = await listDirectory(node.path, node.depth + 1)
			node.loaded = true
			node.expanded = previous.expanded
			await restoreLoadedDirectories(node.children, state)
		}),
	)
}

async function refreshTree() {
	const directoryState = collectDirectoryState(rootNodes.value)
	const nextRoot = await listDirectory('', 0)
	await restoreLoadedDirectories(nextRoot, directoryState)
	const scrollTop = treeScrollElement.value?.scrollTop ?? 0
	rootNodes.value = nextRoot
	await nextTick()
	if (treeScrollElement.value) treeScrollElement.value.scrollTop = scrollTop
}

function flattenTree(nodes: StudioTreeNode[]): StudioTreeNode[] {
	return nodes.flatMap((node) => [
		node,
		...(node.type === 'directory' && node.expanded ? flattenTree(node.children) : []),
	])
}

const visibleNodes = computed(() => flattenTree(rootNodes.value))

async function loadRoot() {
	treeLoading.value = true
	try {
		rootNodes.value = await listDirectory('', 0)
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		treeLoading.value = false
	}
}

async function reloadCleanDocument(document: StudioDocument) {
	if (document.content !== document.savedContent || document.saving) return
	try {
		const nextContent = await readTextFile(resolvePath(document.path))
		if (document.content !== document.savedContent || document.saving) return
		document.content = nextContent
		document.savedContent = nextContent
	} catch {
		// The path may have been removed or be between the two sides of an atomic rename.
	}
}

async function processFileChanges() {
	const paths = changedPaths
	changedPaths = new Set<string>()
	try {
		await refreshTree()
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}

	await Promise.all(
		documents.value
			.filter((document) => paths.has(document.path))
			.map((document) => reloadCleanDocument(document)),
	)
}

function scheduleFileChanges(paths: string[]) {
	for (const path of paths) changedPaths.add(path)
	if (changeTimer) clearTimeout(changeTimer)
	changeTimer = setTimeout(() => {
		changeTimer = null
		void processFileChanges()
	}, 150)
}

async function stopStudioWatcher() {
	const instanceId = watcherInstanceId
	const registrationId = watcherRegistrationId
	watcherInstanceId = null
	watcherRegistrationId = null
	if (instanceId && registrationId) {
		await unregisterStudioWatcher(instanceId, registrationId).catch(() => undefined)
	}
}

async function startStudioWatcher() {
	const generation = ++watcherGeneration
	await stopStudioWatcher()
	const instanceId = props.instance.id
	const registrationId = await registerStudioWatcher(instanceId)
	if (generation !== watcherGeneration) {
		await unregisterStudioWatcher(instanceId, registrationId).catch(() => undefined)
		return
	}
	watcherInstanceId = instanceId
	watcherRegistrationId = registrationId
}

async function toggleDirectory(node: StudioTreeNode) {
	if (node.loading) return
	if (node.loaded) {
		node.expanded = !node.expanded
		return
	}

	node.loading = true
	try {
		node.children = await listDirectory(node.path, node.depth + 1)
		node.loaded = true
		node.expanded = true
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadDirectoryFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		node.loading = false
	}
}

watch(activePath, async (path) => {
	if (!path) return
	await nextTick()
	const node = Array.from(
		treeScrollElement.value?.querySelectorAll<HTMLElement>('[data-studio-path]') ?? [],
	).find((element) => element.dataset.studioPath === path)
	node?.scrollIntoView({ block: 'nearest' })
})

async function openFile(node: StudioTreeNode) {
	if (node.type !== 'file' || !isSupportedFile(node.name) || fileLoading.value) return

	const existingDocument = documents.value.find((document) => document.path === node.path)
	if (existingDocument) {
		if (existingDocument.path === activePath.value) return
		await activateDocument(existingDocument.path)
		return
	}

	fileLoading.value = true
	try {
		const nextContent = await readTextFile(resolvePath(node.path))
		const document: StudioDocument = {
			path: node.path,
			name: node.name,
			content: nextContent,
			savedContent: nextContent,
			saving: false,
		}
		await openDocument(document)
	} catch (error) {
		addNotification({
			title: formatMessage(messages.loadFileFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	} finally {
		fileLoading.value = false
	}
}

async function initialize() {
	instanceRoot.value = await get_full_path(props.instance.id)
	resetDocuments()
	await loadRoot()
}

await initialize()

unlistenStudioFiles = await listenStudioFilesChanged((event) => {
	if (event.instanceId !== watcherInstanceId || event.registrationId !== watcherRegistrationId) {
		return
	}
	scheduleFileChanges(event.paths)
})
await startStudioWatcher()

watch(
	() => props.instance.id,
	async () => {
		await initialize()
		await startStudioWatcher()
	},
)

function handleBeforeUnload(event: BeforeUnloadEvent) {
	if (!hasAnyUnsavedChanges.value) return
	event.preventDefault()
}

window.addEventListener('beforeunload', handleBeforeUnload)
onBeforeUnmount(() => {
	watcherGeneration++
	window.removeEventListener('beforeunload', handleBeforeUnload)
	unlistenStudioFiles?.()
	unlistenStudioFiles = null
	if (changeTimer) clearTimeout(changeTimer)
	void stopStudioWatcher()
})

onBeforeRouteLeave(() => {
	return saveAllDocuments()
})
</script>

<template>
	<div class="flex h-full min-h-0 flex-col">
		<section
			class="grid min-h-0 min-w-0 flex-1 grid-cols-[minmax(13rem,22rem)_minmax(0,1fr)] overflow-hidden rounded-[20px] border border-solid border-surface-4 bg-surface-1 shadow-sm"
		>
			<aside
				class="flex min-h-0 min-w-0 flex-col border-0 border-r border-solid border-surface-4 bg-surface-2"
			>
				<header
					class="flex h-12 shrink-0 items-center gap-2 border-0 border-b border-solid border-surface-4 bg-surface-3 px-3"
				>
					<CodeIcon class="size-5 text-brand" />
					<h1 class="m-0 min-w-0 flex-1 truncate text-sm font-bold text-contrast">
						{{ formatMessage(messages.title) }}
					</h1>
				</header>
				<div
					class="flex h-9 shrink-0 items-center px-3 text-xs font-bold uppercase tracking-wide text-secondary"
				>
					{{ formatMessage(messages.files) }}
				</div>
				<div ref="treeScrollElement" class="min-h-0 flex-1 overflow-y-auto pb-3" role="tree">
					<div v-if="treeLoading" class="px-4 py-3 text-sm text-secondary">
						{{ formatMessage(messages.loadingFile) }}
					</div>
					<button
						v-for="node in visibleNodes"
						:key="node.path"
						type="button"
						role="treeitem"
						:data-studio-path="node.path"
						:aria-expanded="node.type === 'directory' ? node.expanded : undefined"
						:disabled="node.type === 'file' && !isSupportedFile(node.name)"
						class="flex h-8 w-full min-w-0 items-center gap-1 border-0 bg-transparent pr-3 text-left text-sm text-primary hover:bg-surface-3 disabled:cursor-default disabled:opacity-45"
						:class="{ 'bg-brand-highlight !text-contrast': node.path === activeDocument?.path }"
						:style="{ paddingLeft: `${node.depth * 16 + 8}px` }"
						@click="node.type === 'directory' ? toggleDirectory(node) : openFile(node)"
					>
						<template v-if="node.type === 'directory'">
							<ChevronDownIcon v-if="node.expanded" class="size-4 shrink-0" />
							<ChevronRightIcon v-else class="size-4 shrink-0" />
						</template>
						<span v-else class="size-4 shrink-0" />
						<FolderIcon v-if="node.type === 'directory'" class="size-4 shrink-0 text-secondary" />
						<FileCodeIcon v-else class="size-4 shrink-0 text-secondary" />
						<span class="truncate">{{ node.name }}</span>
					</button>
				</div>
			</aside>

			<div class="flex min-h-0 min-w-0 flex-col bg-surface-2">
				<header
					class="flex h-12 shrink-0 items-center overflow-x-auto border-0 border-b border-solid border-surface-4 bg-surface-3"
				>
					<StudioEditorTabs
						:documents="documents"
						:active-path="activePath"
						@activate="activateDocument"
						@close="closeDocument"
					/>
					<div class="ml-auto flex shrink-0 items-center gap-1 px-2">
						<ButtonStyled v-if="hasUnsavedChanges" size="small" type="transparent">
							<button type="button" @click="discardChanges">
								<XIcon class="size-4" />
								{{ formatMessage(messages.discardChanges) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="activeDocument" size="small" color="brand">
							<button
								type="button"
								:disabled="!hasUnsavedChanges || activeDocument.saving"
								@click="saveActiveFile"
							>
								<SaveIcon class="size-4" />
								{{ formatMessage(commonMessages.saveButton) }}
							</button>
						</ButtonStyled>
						<ButtonStyled size="small" type="outlined">
							<button
								type="button"
								@click="router.push({ name: 'Files', params: { id: instance.id } })"
							>
								{{ formatMessage(messages.backToFiles) }}
							</button>
						</ButtonStyled>
					</div>
				</header>
				<nav
					v-if="activeDocument"
					class="flex h-8 shrink-0 items-center gap-1 overflow-x-auto border-0 border-b border-solid border-surface-4 bg-surface-2 px-3 text-xs text-secondary"
					:aria-label="activeDocument.path"
				>
					<template v-for="(segment, index) in breadcrumbSegments" :key="`${segment}-${index}`">
						<ChevronRightIcon v-if="index > 0" class="size-3.5 shrink-0" />
						<FolderIcon v-if="index < breadcrumbSegments.length - 1" class="size-3.5 shrink-0" />
						<FileCodeIcon v-else class="size-3.5 shrink-0" />
						<span
							class="whitespace-nowrap"
							:class="{ 'text-contrast': index === breadcrumbSegments.length - 1 }"
						>
							{{ segment }}
						</span>
					</template>
				</nav>

				<div class="relative min-h-0 min-w-0 flex-1">
					<div
						v-if="fileLoading"
						class="absolute inset-0 z-[2] flex items-center justify-center bg-surface-2 text-sm text-secondary"
					>
						{{ formatMessage(messages.loadingFile) }}
					</div>
					<StudioEditor
						v-if="activeDocument"
						:key="activeDocument.path"
						:file-path="activeDocument.path"
						:content="activeDocument.content"
						:language="editorLanguage"
						:read-only="activeDocument.saving"
						@update:content="updateActiveContent"
						@save="saveActiveFile"
						@blur="saveActiveFile"
					/>
					<div v-else class="flex size-full items-center justify-center p-8 text-center">
						<div class="flex max-w-md flex-col items-center gap-3">
							<CodeIcon class="size-14 text-secondary" />
							<h2 class="m-0 text-xl font-bold text-contrast">
								{{ formatMessage(messages.emptyTitle) }}
							</h2>
							<p class="m-0 text-sm leading-6 text-secondary">
								{{ formatMessage(messages.emptyDescription) }}
							</p>
						</div>
					</div>
				</div>
			</div>
		</section>
	</div>
</template>
