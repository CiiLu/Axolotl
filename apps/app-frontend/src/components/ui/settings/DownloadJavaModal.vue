<script setup>
import { CoffeeIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { trackEvent } from '@/helpers/analytics'
import { auto_install_java, list_java_distribution_versions } from '@/helpers/jre'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	downloadJava: { id: 'app.settings.java.download.title', defaultMessage: 'Download Java' },
	selectVersion: { id: 'app.settings.java.download.select-version', defaultMessage: 'Select a Java version:' },
	loadingVersions: { id: 'app.settings.java.download.loading', defaultMessage: 'Loading versions...' },
	noVersionsFound: { id: 'app.settings.java.download.no-versions', defaultMessage: 'No versions available.' },
	versionLabel: { id: 'app.settings.java.download.version-label', defaultMessage: 'Java {version}' },
})

const emit = defineEmits(['downloaded'])

const modal = ref(null)
const versions = ref([])
const loadingVersions = ref(false)
const downloadingVersion = ref(null)

defineExpose({
	show: async () => {
		versions.value = []
		downloadingVersion.value = null
		loadingVersions.value = true
		modal.value.show()

		const result = await list_java_distribution_versions('zulu').catch(handleError)
		versions.value = (result || []).sort((a, b) => a - b)
		loadingVersions.value = false
	},
})

async function downloadVersion(version) {
	downloadingVersion.value = version
	trackEvent('JavaDownload', { version })

	const path = await auto_install_java(version).catch(handleError)
	downloadingVersion.value = null

	if (path) {
		modal.value.hide()
		emit('downloaded', path, version)
	}
}
</script>
<template>
	<ModalWrapper ref="modal" :header="formatMessage(messages.downloadJava)" :show-ad-on-close="false">
		<div class="flex flex-col gap-4 min-h-32">
			<p class="text-sm text-secondary">
				{{ formatMessage(messages.selectVersion) }}
			</p>

			<div v-if="loadingVersions" class="flex items-center gap-2 text-sm text-secondary py-4">
				<SpinnerIcon class="animate-spin h-4 w-4" />
				{{ formatMessage(messages.loadingVersions) }}
			</div>

			<div v-else-if="versions.length === 0" class="text-sm text-secondary py-4">
				{{ formatMessage(messages.noVersionsFound) }}
			</div>

			<div v-else class="grid grid-cols-4 gap-2">
				<button
					v-for="ver in versions"
					:key="ver"
					class="flex items-center gap-2 px-3 py-2.5 rounded-lg border border-button-border bg-button-bg hover:border-accent transition-colors cursor-pointer"
					:class="{ 'opacity-50 pointer-events-none': downloadingVersion !== null }"
					:disabled="downloadingVersion !== null"
					@click="downloadVersion(ver)"
				>
					<SpinnerIcon v-if="downloadingVersion === ver" class="animate-spin h-4 w-4 shrink-0" />
					<CoffeeIcon v-else class="h-4 w-4 shrink-0" />
					<span class="font-semibold text-sm tabular-nums">{{ formatMessage(messages.versionLabel, { version: ver }) }}</span>
				</button>
			</div>

			<div class="flex justify-end pt-2 border-t border-button-border">
				<ButtonStyled type="outlined">
					<button class="!shadow-none !border-surface-4 !border" :disabled="downloadingVersion !== null" @click="modal.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</ModalWrapper>
</template>
