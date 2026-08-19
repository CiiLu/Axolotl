<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import EditorWorker from '../../../../node_modules/monaco-editor/esm/vs/editor/editor.worker.js?worker'
import JsonWorker from '../../../../node_modules/monaco-editor/esm/vs/language/json/json.worker.js?worker'
import type * as Monaco from 'monaco-editor'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = defineProps<{
	content: string
	filePath: string
	language: string
	readOnly?: boolean
}>()

const emit = defineEmits<{
	'update:content': [content: string]
	save: []
	blur: []
}>()

const messages = defineMessages({
	loading: {
		id: 'instance.files.studio.editor-loading',
		defaultMessage: 'Loading editor...',
	},
})

const { formatMessage } = useVIntl()
const editorElement = ref<HTMLElement | null>(null)
const loading = ref(true)

let monaco: typeof Monaco | null = null
let editor: Monaco.editor.IStandaloneCodeEditor | null = null
let model: Monaco.editor.ITextModel | null = null
let contentSubscription: Monaco.IDisposable | null = null
let resizeObserver: ResizeObserver | null = null
let themeObserver: MutationObserver | null = null
let applyingExternalContent = false
let disposed = false

function cssVariable(name: string): string {
	return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function applyTheme() {
	if (!monaco) return
	const isLight = document.documentElement.classList.contains('light-mode')
	monaco.editor.defineTheme('axolotl-studio', {
		base: isLight ? 'vs' : 'vs-dark',
		inherit: true,
		rules: [],
		colors: {
			'editor.background': cssVariable('--surface-2'),
			'editor.foreground': cssVariable('--color-base'),
			'editorGutter.background': cssVariable('--surface-2'),
			'editorLineNumber.foreground': cssVariable('--color-secondary'),
			'editor.lineHighlightBackground': cssVariable('--surface-3'),
			'editorCursor.foreground': cssVariable('--color-brand'),
		},
	})
	monaco.editor.setTheme('axolotl-studio')
}

function registerStudioLanguages() {
	if (!monaco) return

	if (!monaco.languages.getLanguages().some(({ id }) => id === 'toml')) {
		monaco.languages.register({ id: 'toml', extensions: ['.toml'] })
		monaco.languages.setMonarchTokensProvider('toml', {
			tokenizer: {
				root: [
					[/#.*/, 'comment'],
					[/\[\[?.*?\]\]?/, 'type.identifier'],
					[/^[\w.-]+(?=\s*=)/, 'key'],
					[/"([^"\\]|\\.)*"/, 'string'],
					[/'[^']*'/, 'string'],
					[/\b(true|false)\b/, 'keyword'],
					[/[-+]?\b\d+(\.\d+)?\b/, 'number'],
				],
			},
		})
	}

	if (!monaco.languages.getLanguages().some(({ id }) => id === 'properties')) {
		monaco.languages.register({ id: 'properties', extensions: ['.properties'] })
		monaco.languages.setMonarchTokensProvider('properties', {
			tokenizer: {
				root: [
					[/^[#!].*$/, 'comment'],
					[/^[^\s:=]+(?=\s*[:=])/, 'key'],
					[/[:=]/, 'delimiter'],
					[/\\./, 'string.escape'],
				],
			},
		})
	}
}

function createModel() {
	if (!monaco || !editor) return
	contentSubscription?.dispose()
	model?.dispose()
	model = monaco.editor.createModel(
		props.content,
		props.language,
		monaco.Uri.parse(
			`axolotl-instance://studio/${props.filePath.split('/').map(encodeURIComponent).join('/')}`,
		),
	)
	editor.setModel(model)
	contentSubscription = model.onDidChangeContent(() => {
		if (!applyingExternalContent) emit('update:content', model?.getValue() ?? '')
	})
}

onMounted(async () => {
	self.MonacoEnvironment = {
		getWorker(_moduleId: string, label: string) {
			return label === 'json' ? new JsonWorker() : new EditorWorker()
		},
	}

	monaco = await import('monaco-editor')
	await import('../../../../node_modules/monaco-editor/esm/vs/languages/definitions/yaml/register.js')
	if (disposed) return
	registerStudioLanguages()
	applyTheme()

	if (!editorElement.value) return
	editor = monaco.editor.create(editorElement.value, {
		automaticLayout: false,
		fontSize: 14,
		fontLigatures: true,
		minimap: { enabled: true },
		padding: { top: 12 },
		readOnly: props.readOnly,
		renderWhitespace: 'selection',
		scrollBeyondLastLine: false,
		theme: 'axolotl-studio',
	})
	editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => emit('save'))
	editor.onDidBlurEditorWidget(() => emit('blur'))
	createModel()

	resizeObserver = new ResizeObserver(() => editor?.layout())
	resizeObserver.observe(editorElement.value)
	themeObserver = new MutationObserver(applyTheme)
	themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
	loading.value = false
})

watch(
	() => props.filePath,
	() => createModel(),
)

watch(
	() => props.content,
	(content) => {
		if (!model || model.getValue() === content) return
		applyingExternalContent = true
		model.setValue(content)
		applyingExternalContent = false
	},
)

watch(
	() => props.language,
	(language) => {
		if (monaco && model) monaco.editor.setModelLanguage(model, language)
	},
)

watch(
	() => props.readOnly,
	(readOnly) => editor?.updateOptions({ readOnly }),
)

onBeforeUnmount(() => {
	disposed = true
	contentSubscription?.dispose()
	resizeObserver?.disconnect()
	themeObserver?.disconnect()
	editor?.dispose()
	model?.dispose()
})
</script>

<template>
	<div class="relative size-full min-h-0 min-w-0 bg-surface-2">
		<div
			v-if="loading"
			class="absolute inset-0 z-[1] flex items-center justify-center text-sm text-secondary"
		>
			{{ formatMessage(messages.loading) }}
		</div>
		<div ref="editorElement" class="size-full min-h-0 min-w-0" />
	</div>
</template>
