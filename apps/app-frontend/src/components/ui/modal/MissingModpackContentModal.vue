<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		:fade="remaining > 0 ? 'warning' : 'standard'"
		max-width="760px"
		scrollable
	>
		<div class="flex flex-col gap-4">
			<Admonition
				:type="continuing || remaining === 0 ? 'success' : 'warning'"
				:header="
					continuing
						? formatMessage(messages.continuing)
						: formatMessage(messages.remaining, { count: remaining })
				"
			>
				<template #icon>
					<SpinnerIcon v-if="loading || continuing" class="size-5 animate-spin" />
					<CheckIcon v-else-if="remaining === 0" class="size-5 text-green" />
					<DownloadIcon v-else class="size-5" />
				</template>
				{{
					continuing
						? formatMessage(messages.continuingBody)
						: formatMessage(messages.body)
				}}
			</Admonition>

			<div v-if="files.length" class="flex flex-col overflow-hidden rounded-lg border border-surface-5">
				<div
					v-for="file in files"
					:key="file.itemId"
					class="flex flex-col gap-3 border-0 border-b border-solid border-surface-5 bg-surface-2 p-4 last:border-b-0"
				>
					<div class="flex min-w-0 items-start justify-between gap-3">
						<div class="min-w-0">
							<div class="break-all font-medium text-contrast">{{ file.path }}</div>
							<div class="mt-1 flex flex-wrap gap-2 text-sm text-secondary">
								<span>{{ formatMessage(messages.expectedSize, { size: formatBytes(file.expectedSize) }) }}</span>
								<span>·</span>
								<span>{{ attemptText(file) }}</span>
								<span v-if="file.browserUrls.length > 1">·</span>
								<span v-if="file.browserUrls.length > 1">
									{{ formatMessage(messages.fallbacks, { count: file.browserUrls.length - 1 }) }}
								</span>
							</div>
							<div v-if="file.browserUrls[0]" class="mt-1 truncate text-xs text-secondary">
								<code v-tooltip="file.browserUrls[0]">{{ file.browserUrls[0] }}</code>
							</div>
							<div v-if="file.lastError" class="mt-1 text-sm text-red">
								{{ file.lastError }}
							</div>
						</div>
						<Badge :color="statusColor(file.status)" :type="statusLabel(file.status)" />
					</div>
					<div class="flex flex-wrap gap-2">
						<ButtonStyled color="brand" size="small">
							<button :disabled="isBusy(file.itemId)" @click="retryOne(file.itemId)">
								<SpinnerIcon v-if="isBusy(file.itemId)" class="animate-spin" />
								<RefreshCwIcon v-else />
								{{ formatMessage(messages.retry) }}
							</button>
						</ButtonStyled>
						<ButtonStyled v-if="file.browserUrls.length" type="outlined" size="small">
							<button :disabled="isBusy(file.itemId)" @click="openBrowser(file.browserUrls[0])">
								<ExternalIcon />{{ formatMessage(messages.browserDownload) }}
							</button>
						</ButtonStyled>
						<ButtonStyled type="outlined" size="small">
							<button :disabled="isBusy(file.itemId)" @click="selectLocal(file.itemId)">
								<UploadIcon />{{ formatMessage(messages.chooseFile) }}
							</button>
						</ButtonStyled>
					</div>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex flex-wrap justify-end gap-2">
				<ButtonStyled type="outlined">
					<button @click="modal?.hide()">{{ formatMessage(commonMessages.closeButton) }}</button>
				</ButtonStyled>
				<ButtonStyled v-if="remaining > 0" color="brand">
					<button :disabled="loading || busy.size > 0" @click="retryAll">
						<RefreshCwIcon />{{ formatMessage(messages.retryAll) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	DownloadIcon,
	ExternalIcon,
	RefreshCwIcon,
	SpinnerIcon,
	UploadIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Badge,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onUnmounted, ref } from 'vue'

import { install_job_listener } from '@/helpers/events'
import {
	install_job_import_missing_file,
	install_job_missing_files,
	install_job_resume,
	install_job_retry_missing_file,
	type InstallJobSnapshot,
	type MissingModpackContentView,
} from '@/helpers/install'

type MissingFile = MissingModpackContentView['files'][number]

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const { handleError } = injectNotificationManager()
const modal = ref<InstanceType<typeof NewModal>>()
const jobId = ref<string | null>(null)
const content = ref<MissingModpackContentView>({ remaining: 0, files: [] })
const loading = ref(false)
const continuing = ref(false)
const busy = ref(new Set<string>())
let unlisten: (() => void) | null = null

const files = computed(() => content.value.files)
const remaining = computed(() => content.value.remaining)

const messages = defineMessages({
	header: { id: 'app.downloads.missing-content.header', defaultMessage: 'Complete missing files' },
	body: {
		id: 'app.downloads.missing-content.body',
		defaultMessage: 'Each local file is verified before it can replace the required instance file.',
	},
	remaining: {
		id: 'app.downloads.missing-content.remaining',
		defaultMessage: '{count, plural, one {# file still needs to be completed} other {# files still need to be completed}}',
	},
	continuing: {
		id: 'app.downloads.missing-content.continuing',
		defaultMessage: 'Continuing installation',
	},
	continuingBody: {
		id: 'app.downloads.missing-content.continuing-body',
		defaultMessage: 'All required files are ready. The launcher is verifying them again and continuing installation.',
	},
	expectedSize: {
		id: 'app.downloads.missing-content.expected-size',
		defaultMessage: 'Expected size: {size}',
	},
	fallbacks: {
		id: 'app.downloads.missing-content.fallbacks',
		defaultMessage: '{count} fallback links',
	},
	retry: { id: 'app.downloads.missing-content.retry', defaultMessage: 'Retry download' },
	retryAll: {
		id: 'app.downloads.missing-content.retry-all',
		defaultMessage: 'Retry all missing files',
	},
	browserDownload: {
		id: 'app.downloads.missing-content.browser-download',
		defaultMessage: 'Browser download',
	},
	chooseFile: {
		id: 'app.downloads.missing-content.choose-file',
		defaultMessage: 'Choose local file',
	},
	attempts: {
		id: 'app.downloads.missing-content.attempts',
		defaultMessage: 'Attempt {attempt}/{max}',
	},
	noAttempts: {
		id: 'app.downloads.missing-content.no-attempts',
		defaultMessage: 'No attempt information',
	},
})

const statusMessages = defineMessages({
	failed: { id: 'app.downloads.item-status.failed', defaultMessage: 'Failed' },
	verifying: { id: 'app.downloads.item-status.verifying', defaultMessage: 'Verifying' },
	writing: { id: 'app.downloads.item-status.writing', defaultMessage: 'Writing' },
	completed: { id: 'app.downloads.item-status.completed', defaultMessage: 'Completed' },
	downloading: { id: 'app.downloads.item-status.downloading', defaultMessage: 'Downloading' },
	queued: { id: 'app.downloads.status.queued', defaultMessage: 'Queued' },
})

async function show(job: InstallJobSnapshot) {
	jobId.value = job.job_id
	continuing.value = false
	content.value = {
		remaining: job.items.filter((item) => item.status === 'failed').length,
		files: [],
	}
	modal.value?.show()
	await refresh()
	if (!unlisten) {
		unlisten = await install_job_listener((update: InstallJobSnapshot) => {
			if (update.job_id !== jobId.value) return
			if (update.status === 'waiting_for_user') void refresh()
			else if (update.status === 'queued' || update.status === 'running') continuing.value = true
		})
	}
}

async function refresh() {
	if (!jobId.value || continuing.value) return
	loading.value = true
	try {
		content.value = await install_job_missing_files(jobId.value)
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function runItem(itemId: string, action: () => Promise<InstallJobSnapshot>) {
	busy.value = new Set([...busy.value, itemId])
	try {
		const job = await action()
		if (job.status === 'waiting_for_user') await refresh()
		else continuing.value = true
	} catch (error) {
		handleError(error)
		await refresh()
	} finally {
		const next = new Set(busy.value)
		next.delete(itemId)
		busy.value = next
	}
}

async function retryOne(itemId: string) {
	if (!jobId.value) return
	await runItem(itemId, () => install_job_retry_missing_file(jobId.value!, itemId))
}

async function selectLocal(itemId: string) {
	if (!jobId.value) return
	const selected = await open({ multiple: false })
	const path = selectedPath(selected)
	if (!path) return
	await runItem(itemId, () => install_job_import_missing_file(jobId.value!, itemId, path))
}

function selectedPath(selected: unknown) {
	if (typeof selected === 'string') return selected
	if (
		selected &&
		typeof selected === 'object' &&
		'path' in selected &&
		typeof selected.path === 'string'
	) {
		return selected.path
	}
	return null
}

async function retryAll() {
	if (!jobId.value) return
	loading.value = true
	try {
		await install_job_resume(jobId.value)
		continuing.value = true
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function openBrowser(url: string) {
	try {
		await openUrl(url)
	} catch (error) {
		handleError(error)
	}
}

function isBusy(itemId: string) {
	return busy.value.has(itemId)
}

function attemptText(file: MissingFile) {
	if (file.attempt == null || file.maxAttempts == null) return formatMessage(messages.noAttempts)
	return formatMessage(messages.attempts, { attempt: file.attempt, max: file.maxAttempts })
}

function statusLabel(status: MissingFile['status']) {
	return status in statusMessages
		? formatMessage(statusMessages[status as keyof typeof statusMessages])
		: status
}

function statusColor(status: MissingFile['status']): 'green' | 'red' | 'orange' | 'blue' | 'gray' {
	if (status === 'completed') return 'green'
	if (status === 'failed') return 'red'
	if (status === 'verifying' || status === 'writing') return 'orange'
	return 'blue'
}

onUnmounted(() => unlisten?.())

defineExpose({ show })
</script>
