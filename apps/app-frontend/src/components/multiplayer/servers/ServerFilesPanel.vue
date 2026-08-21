<script setup lang="ts">
import type { EditingFile, FileItem } from '@modrinth/ui'
import {
	commonMessages,
	defineMessages,
	FilePageLayout,
	injectNotificationManager,
	provideFileManager,
	useVIntl,
} from '@modrinth/ui'
import { save } from '@tauri-apps/plugin-dialog'
import {
	copyFile,
	mkdir,
	readDir,
	readFile as readFileBytes,
	readTextFile,
	remove,
	rename,
	stat,
	writeTextFile,
} from '@tauri-apps/plugin-fs'
import { computed, ref, watch } from 'vue'

import type { ServerView } from '@/composables/useServers'
import { highlightInFolder } from '@/helpers/utils'

const props = defineProps<{
	server: ServerView
}>()

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	saveAs: {
		id: 'app.servers.files.save-as',
		defaultMessage: 'Save as...',
	},
	busyTooltip: {
		id: 'app.servers.files.busy-tooltip',
		defaultMessage: 'Stop the server to modify files',
	},
})

const items = ref<FileItem[]>([])
const loading = ref(true)
const error = ref<Error | null>(null)
const currentPath = ref('')
const editingFile = ref<EditingFile | null>(null)

const serverRoot = computed(() => props.server.path)
const isBusy = computed(() => props.server.running)

function resolvePath(relativePath: string): string {
	const clean = relativePath.startsWith('/') ? relativePath.slice(1) : relativePath
	return clean ? `${serverRoot.value}/${clean}` : serverRoot.value
}

async function listDirectory(dirPath: string): Promise<FileItem[]> {
	const absPath = resolvePath(dirPath)
	const entries = await readDir(absPath)

	const results = await Promise.all(
		entries.map(async (entry) => {
			const entryAbsPath = `${absPath}/${entry.name}`
			let metadata
			try {
				metadata = await stat(entryAbsPath)
			} catch {
				return null
			}
			const item: FileItem = {
				name: entry.name,
				type: entry.isDirectory ? 'directory' : 'file',
				path: dirPath ? `${dirPath}/${entry.name}` : entry.name,
				modified: metadata.mtime ? Math.floor(metadata.mtime.getTime() / 1000) : 0,
				created: metadata.birthtime ? Math.floor(metadata.birthtime.getTime() / 1000) : 0,
			}
			if (!entry.isDirectory) {
				item.size = metadata.size
			}
			if (entry.isDirectory) {
				try {
					const children = await readDir(entryAbsPath)
					item.count = children.length
				} catch {
					item.count = 0
				}
			}
			return item
		}),
	)
	return results.filter((item): item is FileItem => item !== null)
}

async function refresh() {
	loading.value = true
	error.value = null
	try {
		items.value = await listDirectory(currentPath.value)
	} catch (e) {
		error.value = e instanceof Error ? e : new Error(String(e))
		items.value = []
	} finally {
		loading.value = false
	}
}

function navigateTo(path: string) {
	currentPath.value = path.startsWith('/') ? path.slice(1) : path
	void refresh()
}

function startEditing(file: EditingFile) {
	editingFile.value = file
}

function stopEditing() {
	editingFile.value = null
}

function notifyFailure(label: string, e: unknown) {
	addNotification({
		title: label,
		text: e instanceof Error ? e.message : '',
		type: 'error',
	})
}

async function handleCreateItem(name: string, type: 'file' | 'directory') {
	const targetPath = currentPath.value ? `${currentPath.value}/${name}` : name
	const absPath = resolvePath(targetPath)
	try {
		if (type === 'directory') {
			await mkdir(absPath)
		} else {
			await writeTextFile(absPath, '')
		}
		await refresh()
	} catch (e) {
		notifyFailure(formatMessage(commonMessages.createFailedLabel), e)
	}
}

async function handleRenameItem(path: string, newName: string) {
	const oldAbs = resolvePath(path)
	const parentDir = path.includes('/') ? path.substring(0, path.lastIndexOf('/')) : ''
	const newPath = parentDir ? `${parentDir}/${newName}` : newName
	try {
		await rename(oldAbs, resolvePath(newPath))
		await refresh()
	} catch (e) {
		notifyFailure(formatMessage(commonMessages.renameFailedLabel), e)
	}
}

async function handleMoveItem(source: string, destination: string) {
	try {
		await rename(resolvePath(source), resolvePath(destination))
		await refresh()
	} catch (e) {
		notifyFailure(formatMessage(commonMessages.moveFailedLabel), e)
	}
}

async function handleDeleteItem(path: string, recursive: boolean) {
	try {
		await remove(resolvePath(path), { recursive })
		await refresh()
	} catch (e) {
		notifyFailure(formatMessage(commonMessages.deleteFailedLabel), e)
	}
}

async function handleReadFile(path: string): Promise<string> {
	return await readTextFile(resolvePath(path))
}

async function handleReadFileAsBlob(path: string): Promise<Blob> {
	const bytes = await readFileBytes(resolvePath(path))
	return new Blob([bytes])
}

async function handleWriteFile(path: string, content: string) {
	await writeTextFile(resolvePath(path), content)
}

async function handleDownloadFile(path: string, fileName: string) {
	const outputPath = await save({ defaultPath: fileName })
	if (!outputPath) return
	await copyFile(resolvePath(path), outputPath)
}

watch(
	() => props.server.path,
	async () => {
		currentPath.value = ''
		await refresh()
	},
	{ immediate: true },
)

provideFileManager({
	items,
	loading,
	error,
	currentPath,
	navigateTo,
	editingFile,
	startEditing,
	stopEditing,
	createItem: handleCreateItem,
	renameItem: handleRenameItem,
	moveItem: handleMoveItem,
	deleteItem: handleDeleteItem,
	readFile: handleReadFile,
	readFileAsBlob: handleReadFileAsBlob,
	writeFile: handleWriteFile,
	downloadFile: handleDownloadFile,
	refresh,
	isBusy,
	busyTooltip: computed(() => (isBusy.value ? formatMessage(messages.busyTooltip) : undefined)),
	basePath: serverRoot,
	openInFolder: (path: string) => highlightInFolder(path),
	downloadButtonLabel: formatMessage(messages.saveAs),
})
</script>

<template>
	<div class="min-h-0 w-full">
		<FilePageLayout :show-refresh-button="true" />
	</div>
</template>
