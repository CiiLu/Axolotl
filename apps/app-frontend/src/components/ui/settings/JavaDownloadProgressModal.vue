<script setup>
import { CoffeeIcon, DownloadIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { NewModal, ButtonStyled, defineMessages, useVIntl, commonMessages } from '@modrinth/ui'
import { ref } from 'vue'
import { cancel_java_download } from '@/helpers/jre'

const { formatMessage } = useVIntl()

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
	cancelling: {
		id: 'app.settings.java.download.cancelling',
		defaultMessage: 'Cancelling...',
	},
})

const modal = ref(null)

const distributionName = ref('')
const javaVersion = ref(0)
const statusText = ref('')

defineExpose({
	show(distribution, version) {
		distributionName.value = distribution.name || distribution.id || ''
		javaVersion.value = version
		statusText.value = formatMessage(messages.preparingDownload)
		modal.value.show()
	},
	updateStatus(text) {
		statusText.value = text
	},
	complete(path) {
		modal.value.hide()
	},
	close() {
		modal.value.hide()
	},
})

const cancelling = ref(false)

function handleBackground() {
	modal.value.hide()
}

async function handleCancel() {
	cancelling.value = true
	await cancel_java_download().catch(() => {})
	// The download will error with "cancelled by user"
	// Modal will be closed by the parent when the download call returns
}
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
			>
				<CoffeeIcon class="h-8 w-8 text-accent" />
			</div>
			<div class="text-center">
				<div class="font-semibold text-contrast">
					{{ formatMessage(messages.downloadingInfo, { version: javaVersion, distribution: distributionName }) }}
				</div>
				<div class="flex items-center justify-center gap-2 mt-2 text-sm text-secondary">
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
