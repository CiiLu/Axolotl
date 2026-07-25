<script setup>
import {
	ArrowLeftIcon,
	CoffeeIcon,
	DownloadIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
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
import { auto_install_java_distribution, list_java_distribution_versions } from '@/helpers/jre'
import JavaDownloadProgressModal from '@/components/ui/settings/JavaDownloadProgressModal.vue'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	downloadJava: {
		id: 'app.settings.java.download.title',
		defaultMessage: 'Download Java',
	},
	selectDistribution: {
		id: 'app.settings.java.download.select-distribution',
		defaultMessage: 'Choose a distribution:',
	},
	selectVersion: {
		id: 'app.settings.java.download.select-version',
		defaultMessage: 'Select a Java version for {distro}:',
	},
	backToDistributions: {
		id: 'app.settings.java.download.back',
		defaultMessage: 'Back to distributions',
	},
	downloading: {
		id: 'app.settings.java.download.downloading',
		defaultMessage: 'Downloading Java {version}...',
	},
	downloadingLabel: {
		id: 'app.settings.java.download.downloading.status',
		defaultMessage: 'Downloading...',
	},
	extractingText: {
		id: 'app.settings.java.download.extracting.status',
		defaultMessage: 'Extracting files...',
	},
	loadingVersions: {
		id: 'app.settings.java.download.loading',
		defaultMessage: 'Loading versions...',
	},
	noVersionsFound: {
		id: 'app.settings.java.download.no-versions',
		defaultMessage: 'No versions available for this distribution.',
	},
})

const emit = defineEmits(['downloaded'])

const modal = ref(null)
const progressModal = ref(null)
const selectedDistribution = ref(null)
const distributionVersions = ref([])
const loadingDistroVersions = ref(false)
const downloadingVersion = ref(null)

const distributions = [
	{
		id: 'adoptium',
		name: 'Eclipse Temurin',
		vendor: 'Adoptium',
		desc: 'HotSpot JVM, TCK-tested, most popular',
		jvm: 'HotSpot',
	},
	{
		id: 'semeru',
		name: 'IBM Semeru',
		vendor: 'IBM',
		desc: 'OpenJ9 JVM, low memory, fast startup',
		jvm: 'OpenJ9',
	},
	{
		id: 'zulu',
		name: 'Azul Zulu',
		vendor: 'Azul',
		desc: 'HotSpot JVM, enterprise support, compact',
		jvm: 'HotSpot',
	},
]

defineExpose({
	show: () => {
		selectedDistribution.value = null
		distributionVersions.value = []
		loadingDistroVersions.value = false
		downloadingVersion.value = null
		modal.value.show()
	},
})

async function selectDistribution(distro) {
	selectedDistribution.value = distro
	loadingDistroVersions.value = true
	distributionVersions.value = []

	const versions = await list_java_distribution_versions(distro.id).catch(handleError)
	distributionVersions.value = versions || []
	loadingDistroVersions.value = false

	trackEvent('JavaDownloadDistroSelect', { distribution: distro.id })
}

function backToDistributions() {
	selectedDistribution.value = null
	distributionVersions.value = []
}

async function downloadVersion(version) {
	const distro = selectedDistribution.value
	if (!distro) return

	downloadingVersion.value = version
	trackEvent('JavaDownload', { distribution: distro.id, version })

	progressModal.value.show(distro, version)
	progressModal.value.updateStatus(formatMessage(messages.downloadingLabel))

	try {
		const path = await auto_install_java_distribution(distro.id, version)
		downloadingVersion.value = null

		if (path) {
			progressModal.value.complete(path)
			emit('downloaded', path, version)
		} else {
			progressModal.value.close()
		}
	} catch (e) {
		downloadingVersion.value = null
		progressModal.value.close()
		// Only show error if it's not a user cancellation
		const msg = String(e)
		if (!msg.includes('cancelled') && !msg.includes('canceled')) {
			handleError(e)
		}
	}
}
</script>
<template>
	<ModalWrapper ref="modal" :header="formatMessage(messages.downloadJava)" :show-ad-on-close="false">
		<JavaDownloadProgressModal ref="progressModal" />
		<div class="flex flex-col gap-4 min-h-40">
			<!-- Step 1: Distribution selection -->
			<template v-if="!selectedDistribution">
				<p class="text-sm text-secondary">
					{{ formatMessage(messages.selectDistribution) }}
				</p>
				<div class="flex flex-col gap-2">
					<div
						v-for="distro in distributions"
						:key="distro.id"
						class="flex items-center gap-3 p-3 rounded-lg border border-button-border bg-button-bg hover:border-accent transition-colors cursor-pointer"
						:class="{ 'opacity-50 pointer-events-none': downloadingVersion !== null }"
						@click="selectDistribution(distro)"
					>
						<div
							class="w-9 h-9 flex items-center justify-center rounded-full bg-button-bg border border-button-border shrink-0"
						>
							<CoffeeIcon class="h-5 w-5" />
						</div>
						<div class="flex-1 min-w-0">
							<div class="font-semibold text-sm">{{ distro.name }}</div>
							<div class="text-xs text-secondary">{{ distro.vendor }} — {{ distro.desc }}</div>
						</div>
					</div>
				</div>
			</template>

			<!-- Step 2: Version selection -->
			<template v-else>
				<button
					class="flex items-center gap-1 text-sm text-secondary hover:text-contrast transition-colors -ml-1 px-1 py-0.5"
					@click="backToDistributions"
				>
					<ArrowLeftIcon class="h-3.5 w-3.5" />
					{{ formatMessage(messages.backToDistributions) }}
				</button>
				<p class="text-sm text-secondary">
					{{ formatMessage(messages.selectVersion, { distro: selectedDistribution.name }) }}
				</p>

				<div v-if="loadingDistroVersions" class="flex items-center gap-2 text-sm text-secondary py-4">
					<SpinnerIcon class="animate-spin h-4 w-4" />
					{{ formatMessage(messages.loadingVersions) }}
				</div>

				<div v-else-if="distributionVersions.length === 0" class="text-sm text-secondary py-4">
					{{ formatMessage(messages.noVersionsFound) }}
				</div>

				<div v-else class="flex flex-wrap gap-2">
					<button
						v-for="ver in distributionVersions"
						:key="ver"
						class="flex items-center gap-2 px-3 py-2 rounded-lg border border-button-border bg-button-bg hover:border-accent transition-colors cursor-pointer"
						:class="{ 'opacity-50 pointer-events-none': downloadingVersion !== null }"
						:disabled="downloadingVersion !== null"
						@click="downloadVersion(ver)"
					>
						<CoffeeIcon class="h-4 w-4" />
						<span class="font-semibold text-sm tabular-nums">Java {{ ver }}</span>
						<SpinnerIcon
							v-if="downloadingVersion === ver"
							class="animate-spin h-3.5 w-3.5"
						/>
						<DownloadIcon v-else class="h-3.5 w-3.5 text-secondary" />
					</button>
				</div>
			</template>

			<div class="flex justify-end pt-2 border-t border-button-border">
				<ButtonStyled type="outlined">
					<button
						class="!shadow-none !border-surface-4 !border"
						:disabled="downloadingVersion !== null"
						@click="modal.hide()"
					>
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</ModalWrapper>
</template>
