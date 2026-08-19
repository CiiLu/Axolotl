import { computed, ref } from 'vue'

export interface StudioDocument {
	path: string
	name: string
	content: string
	savedContent: string
	saving: boolean
}

export function useStudioDocuments(
	writeDocument: (document: StudioDocument, content: string) => Promise<void>,
	onSaveError: (error: unknown) => void,
) {
	const documents = ref<StudioDocument[]>([])
	const activePath = ref('')
	const savePromises = new Map<string, Promise<boolean>>()

	const activeDocument = computed(
		() => documents.value.find((document) => document.path === activePath.value) ?? null,
	)
	const hasUnsavedChanges = computed(
		() =>
			activeDocument.value !== null &&
			activeDocument.value.content !== activeDocument.value.savedContent,
	)
	const hasAnyUnsavedChanges = computed(() =>
		documents.value.some((document) => document.content !== document.savedContent),
	)

	function saveDocument(document: StudioDocument | null): Promise<boolean> {
		if (!document || document.content === document.savedContent) return Promise.resolve(true)

		const existingPromise = savePromises.get(document.path)
		if (existingPromise) return existingPromise

		document.saving = true
		const contentToSave = document.content
		const savePromise = writeDocument(document, contentToSave)
			.then(() => {
				document.savedContent = contentToSave
				return true
			})
			.catch((error) => {
				onSaveError(error)
				return false
			})
			.finally(() => {
				document.saving = false
				savePromises.delete(document.path)
			})

		savePromises.set(document.path, savePromise)
		return savePromise
	}

	async function activate(path: string) {
		if (path === activePath.value) return true
		if (!(await saveDocument(activeDocument.value))) return false
		if (!documents.value.some((document) => document.path === path)) return false
		activePath.value = path
		return true
	}

	async function open(document: StudioDocument) {
		const existing = documents.value.find((candidate) => candidate.path === document.path)
		if (existing) return activate(existing.path)
		if (!(await saveDocument(activeDocument.value))) return false
		documents.value.push(document)
		activePath.value = document.path
		return true
	}

	async function close(path: string) {
		const index = documents.value.findIndex((document) => document.path === path)
		if (index === -1) return false
		if (!(await saveDocument(documents.value[index]))) return false

		const wasActive = activePath.value === path
		const fallbackPath = documents.value[index + 1]?.path ?? documents.value[index - 1]?.path ?? ''
		documents.value.splice(index, 1)
		if (wasActive) activePath.value = fallbackPath
		return true
	}

	function updateActiveContent(content: string) {
		if (activeDocument.value) activeDocument.value.content = content
	}

	function discardActiveChanges() {
		if (activeDocument.value) activeDocument.value.content = activeDocument.value.savedContent
	}

	async function saveActive() {
		return saveDocument(activeDocument.value)
	}

	async function saveAll() {
		const results = await Promise.all(documents.value.map((document) => saveDocument(document)))
		return results.every(Boolean)
	}

	function reset() {
		documents.value = []
		activePath.value = ''
		savePromises.clear()
	}

	return {
		documents,
		activeDocument,
		activePath,
		hasUnsavedChanges,
		hasAnyUnsavedChanges,
		activate,
		open,
		close,
		saveDocument,
		saveActive,
		saveAll,
		updateActiveContent,
		discardActiveChanges,
		reset,
	}
}
