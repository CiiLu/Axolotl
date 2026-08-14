<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { renderHighlightedString } from '@modrinth/utils/highlightjs'
import { computed, onUnmounted, ref } from 'vue'

import type { CrashAnalysisResult } from '@/composables/useCrashAnalysis'

const MAX_AI_CONTENT_CHARS = 250_000
const RENDER_THROTTLE_MS = 120

const modal = ref<InstanceType<typeof NewModal>>()
const { formatMessage } = useVIntl()
const client = injectModrinthClient()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	title: {
		id: 'app.ai-log-analysis.title',
		defaultMessage: 'AI log analysis',
	},
	privacy: {
		id: 'app.ai-log-analysis.privacy',
		defaultMessage:
			'Your log content will be uploaded and sent to a third-party AI service to be analyzed. Treat it as public information.',
	},
	uploading: {
		id: 'app.ai-log-analysis.uploading',
		defaultMessage: 'Uploading your log...',
	},
	copyLink: {
		id: 'app.ai-log-analysis.copy-link',
		defaultMessage: 'Copy link',
	},
	linkCopied: {
		id: 'app.ai-log-analysis.link-copied',
		defaultMessage: 'Link copied to your clipboard',
	},
	uploadFailed: {
		id: 'app.ai-log-analysis.upload-failed',
		defaultMessage: 'Upload failed, analyzing the log content directly without a share link.',
	},
	analyzing: {
		id: 'app.ai-log-analysis.analyzing',
		defaultMessage: 'Analyzing your log with AI...',
	},
	error: {
		id: 'app.ai-log-analysis.error',
		defaultMessage: 'AI analysis failed: {message}',
	},
	logshareUnavailable: {
		id: 'app.ai-log-analysis.logshare-unavailable',
		defaultMessage:
			'LogShare.CN is unavailable, so AI analysis cannot be used (mclo.gs does not support AI analysis). {message}',
	},
	copyResult: {
		id: 'app.ai-log-analysis.copy-result',
		defaultMessage: 'Copy result',
	},
	resultCopied: {
		id: 'app.ai-log-analysis.result-copied',
		defaultMessage: 'Analysis copied to your clipboard',
	},
	cancel: {
		id: 'app.ai-log-analysis.cancel',
		defaultMessage: 'Cancel',
	},
	close: {
		id: 'app.ai-log-analysis.close',
		defaultMessage: 'Close',
	},
})

type AnalysisStatus = 'uploading' | 'streaming' | 'done' | 'error'

const status = ref<AnalysisStatus>('uploading')
const output = ref('')
const errorMessage = ref('')
const shareUrl = ref('')
let abortController: AbortController | null = null
let renderTimer: ReturnType<typeof setTimeout> | null = null

// Streaming re-renders the full markdown with syntax highlighting on every
// chunk; throttle the rendered snapshot so highlight.js does not run per chunk.
const renderedText = ref('')
const renderedOutput = computed(() => renderHighlightedString(renderedText.value))
const isStreaming = computed(() => status.value === 'streaming')

function scheduleRender(): void {
	if (renderTimer !== null) return
	renderTimer = setTimeout(() => {
		renderTimer = null
		renderedText.value = output.value
	}, RENDER_THROTTLE_MS)
}

function flushRender(): void {
	if (renderTimer !== null) {
		clearTimeout(renderTimer)
		renderTimer = null
	}
	renderedText.value = output.value
}

function resetRender(): void {
	flushRender()
	output.value = ''
	renderedText.value = ''
}

function buildContent(analysis: CrashAnalysisResult): string {
	const combined = analysis.sources
		.map((source) => `\n===== ${source.filename} =====\n${source.content}`)
		.join('')
	if (combined.length <= MAX_AI_CONTENT_CHARS) return combined
	return `${combined.slice(0, 100_000)}\n... omitted ...\n${combined.slice(
		combined.length - (MAX_AI_CONTENT_CHARS - 100_000),
	)}`
}

function show(
	analysis: CrashAnalysisResult,
	existing?: { id?: string; url?: string } | null,
): void {
	status.value = 'uploading'
	resetRender()
	errorMessage.value = ''
	shareUrl.value = ''
	abortController = new AbortController()
	modal.value?.show()
	const content = buildContent(analysis)
	if (existing?.id && existing.url) {
		// Reuse the log that was already uploaded by "Export & share".
		shareUrl.value = existing.url
		void streamAI(content, existing.id, abortController)
	} else {
		void run(content)
	}
}

async function run(content: string): Promise<void> {
	const controller = abortController
	const uploadedId = await upload(content, controller)
	await streamAI(content, uploadedId, controller)
}

async function upload(content: string, controller: AbortController | null): Promise<string | null> {
	try {
		const data = await client.logshare.logs_v1.create(content, 'axolotl-ai', {
			kind: 'ai-analysis',
		})
		shareUrl.value = data.url
		return data.id
	} catch (error) {
		console.error('Failed to upload log for AI analysis', error)
		if (controller?.signal.aborted) return null
		addNotification({
			title: formatMessage(messages.uploadFailed),
			type: 'warning',
		})
		return null
	}
}

async function streamAI(
	content: string,
	id: string | null,
	controller: AbortController | null,
): Promise<void> {
	status.value = 'streaming'
	const handlers = {
		onChunk: (chunk: string) => {
			output.value += chunk
			scheduleRender()
		},
		onDone: () => {
			flushRender()
			if (status.value === 'streaming') status.value = 'done'
		},
		onError: (error: { message: string }) => {
			status.value = 'error'
			// AI analysis is only provided by logshare.cn; any failure here means
			// LogShare is unreachable and mclo.gs cannot take over.
			errorMessage.value = formatMessage(messages.logshareUnavailable, {
				message: error.message,
			})
		},
	}
	if (id) {
		await client.logshare.ai_v1.getStream(id, handlers, controller?.signal)
	} else {
		await client.logshare.ai_v1.analyseStream(content, handlers, controller?.signal)
	}
}

function cancel(): void {
	abortController?.abort()
	abortController = null
	if (status.value === 'streaming') {
		flushRender()
		status.value = 'done'
	}
}

async function copyLink(): Promise<void> {
	try {
		await navigator.clipboard.writeText(shareUrl.value)
		addNotification({
			title: formatMessage(messages.linkCopied),
			type: 'success',
		})
	} catch {
		addNotification({
			title: formatMessage(messages.error, {
				message: 'Could not copy the link',
			}),
			type: 'error',
		})
	}
}

async function copyResult(): Promise<void> {
	try {
		await navigator.clipboard.writeText(output.value)
		addNotification({
			title: formatMessage(messages.resultCopied),
			type: 'success',
		})
	} catch {
		addNotification({
			title: formatMessage(messages.error, {
				message: 'Could not copy the result',
			}),
			type: 'error',
		})
	}
}

function handleHide(): void {
	abortController?.abort()
	abortController = null
	if (renderTimer !== null) {
		clearTimeout(renderTimer)
		renderTimer = null
	}
}

onUnmounted(() => {
	abortController?.abort()
	if (renderTimer !== null) {
		clearTimeout(renderTimer)
		renderTimer = null
	}
})

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.title)"
		max-width="720px"
		:disable-close="isStreaming"
		@hide="handleHide"
	>
		<div class="flex flex-col gap-4">
			<Admonition type="warning" :header="formatMessage(messages.analyzing)">
				{{ formatMessage(messages.privacy) }}
			</Admonition>

			<div v-if="shareUrl" class="flex items-center gap-2 rounded-lg bg-surface-2 p-3">
				<ExternalIcon class="h-4 w-4 shrink-0 text-secondary" />
				<a
					:href="shareUrl"
					target="_blank"
					rel="noopener noreferrer"
					class="min-w-0 flex-1 truncate text-primary underline"
				>
					{{ shareUrl }}
				</a>
				<ButtonStyled type="outlined">
					<button @click="copyLink">
						{{ formatMessage(messages.copyLink) }}
					</button>
				</ButtonStyled>
			</div>

			<div v-if="status === 'uploading'" class="text-secondary">
				{{ formatMessage(messages.uploading) }}
			</div>

			<div
				v-else-if="status === 'error'"
				class="flex flex-col gap-2 rounded-lg bg-red-500/10 p-3 text-secondary"
			>
				{{ formatMessage(messages.error, { message: errorMessage }) }}
			</div>

			<div
				v-else-if="output"
				class="markdown-body max-h-[55vh] overflow-y-auto rounded-lg bg-surface-2 p-4"
				v-html="renderedOutput"
			/>
			<div v-else-if="isStreaming" class="text-secondary">
				{{ formatMessage(messages.analyzing) }}
			</div>
		</div>

		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled v-if="isStreaming" type="outlined">
					<button @click="cancel">
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="status === 'done' && output" type="outlined">
					<button @click="copyResult">
						{{ formatMessage(messages.copyResult) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="!isStreaming" color="brand">
					<button @click="modal?.hide()">
						{{ formatMessage(messages.close) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>
