<script setup>
import { CoffeeIcon, DownloadIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { NewModal, ButtonStyled, defineMessages, useVIntl, commonMessages, injectNotificationManager } from '@modrinth/ui'
import { ref, onUnmounted } from 'vue'
import { cancel_java_download } from '@/helpers/jre'
import { loading_listener } from '@/helpers/events'

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	downloadingJava: {
		id: 'app.settings.java.download.progress-title',
		defaultMessage: 'Downloading Java',
	},
	downloadingInfo: {
		id: 'app.settings.java.download.progress-info',
		defaultMessage: 'Downloading Java {version} from {distribution}',
	},
	preparingDownload: {
		id: 'app.settings.java.download.preparing',
		defaultMessage: 'Preparing download...',
	},
	extracting: {
		id: 'app.settings.java.download.extracting',
		defaultMessage: 'Extracting files...',
	},
	background: {
		id: 'app.settings.java.download.background',
		defaultMessage: 'Background',
	},
	backgroundHint: {
		id: 'app.settings.java.download.background-hint',
		defaultMessage: 'Download continues — check the Downloads page for progress.',
	},
	cancelling: {
		id: 'app.settings.java.download.cancelling',
		defaultMessage: 'Cancelling...',
	},
})

const modal = ref(null)

const distributionName = ref('')
const javaVersion = ref(0)
const statusText = ref('')
const progress = ref(0)
const progressMessage = ref('')
const progressBar = ref(false)
let unlistenLoading = null

defineExpose({
	show(distribution, version) {
		distributionName.value = distribution.name || distribution.id || ''
		javaVersion.value = version
		progress.value = 0
		progressBar.value = false
		statusText.value = formatMessage(messages.preparingDownload)
		modal.value.show()

		loading_listener((payload) => {
			if (payload.event?.type === 'java_download' && payload.event?.version === version) {
				progressBar.value = true
				if (payload.fraction !== null && payload.fraction !== undefined) {
					progress.value = Math.round(payload.fraction * 100)
				}
				if (payload.message) {
					progressMessage.value = payload.message
				}
			}
		}).then(fn => { unlistenLoading = fn })
	},
	updateStatus(text) {
		statusText.value = text
	},
	complete(path) {
		if (unlistenLoading) { unlistenLoading(); unlistenLoading = null }
		modal.value.hide()
	},
	close() {
		if (unlistenLoading) { unlistenLoading(); unlistenLoading = null }
		modal.value.hide()
	},
})

const cancelling = ref(false)

function handleBackground() {
	addNotification({
		type: 'info',
		title: formatMessage(messages.downloadingJava),
		text: formatMessage(messages.backgroundHint),
		autoCloseMs: 8000,
	})
	modal.value.hide()
}

async function handleCancel() {
	cancelling.value = true
	await cancel_java_download().catch(() => {})
	// The download will error with "cancelled by user"
	// Modal will be closed by the parent when the download call returns
}

onUnmounted(() => {
	if (unlistenLoading) { unlistenLoading(); unlistenLoading = null }
})
</script>
<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.downloadingJava)"
		:closable="false"
		:close-on-esc="false"
		:close-on-click-outside="false"
		:max-width="'28rem'"
	>
		<div class="flex flex-col items-center gap-4 py-4">
			<div
				class="w-16 h-16 flex items-center justify-center rounded-full bg-button-bg border-2 border-accent animate-pulse"
				aria-hidden="true"
			>
				<CoffeeIcon class="h-8 w-8 text-accent" />
			</div>
			<div class="text-center">
				<div class="font-semibold text-contrast">
					{{ formatMessage(messages.downloadingInfo, { version: javaVersion, distribution: distributionName }) }}
				</div>
				<div v-if="progressBar" class="w-full mt-2">
					<div class="flex justify-between text-xs text-secondary mb-1">
						<span>{{ progressMessage || statusText }}</span>
						<span>{{ progress }}%</span>
					</div>
					<div class="w-full h-1.5 bg-surface-4 rounded-full overflow-hidden">
						<div
							class="h-full bg-accent rounded-full transition-all duration-300 ease-out"
							:style="{ width: progress + '%' }"
						/>
					</div>
				</div>
				<div v-else class="flex items-center justify-center gap-2 mt-2 text-sm text-secondary">
					<SpinnerIcon class="animate-spin h-4 w-4" />
					{{ statusText }}
				</div>
			</div>
			<div class="flex gap-2 mt-2">
				<ButtonStyled type="outlined">
					<button class="!shadow-none !border-surface-4 !border" @click="handleBackground">
						{{ formatMessage(messages.background) }}
					</button>
				</ButtonStyled>
				<ButtonStyled type="outlined" color="red">
					<button class="!shadow-none !border-surface-4 !border" :disabled="cancelling" @click="handleCancel">
						<XIcon class="h-4 w-4" />
						{{ cancelling ? formatMessage(messages.cancelling) : formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>
