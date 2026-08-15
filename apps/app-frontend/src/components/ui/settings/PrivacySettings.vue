<script setup lang="ts">
import {
	Combobox,
	defineMessages,
	getLogShareProvider,
	injectNotificationManager,
	type LogShareProvider,
	setLogShareProvider,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { getPrivacySettings, setDiscordRpcEnabled, setTelemetryEnabled } from '@/helpers/settings'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const privacy = ref(await getPrivacySettings())
const telemetrySaving = ref(false)
const discordSaving = ref(false)
const selectedProvider = ref<LogShareProvider>(getLogShareProvider())

const messages = defineMessages({
	telemetry: {
		id: 'app.settings.privacy.telemetry',
		defaultMessage: 'Allow telemetry',
	},
	telemetryDescription: {
		id: 'app.settings.privacy.telemetry-description',
		defaultMessage:
			'Send an anonymous daily activity signal and sanitized launcher error reports. Minecraft logs and account credentials are never uploaded.',
	},
	discordRpc: {
		id: 'app.settings.privacy.discord-rpc',
		defaultMessage: 'Discord Rich Presence',
	},
	discordRpcDescription: {
		id: 'app.settings.privacy.discord-rpc-description',
		defaultMessage: 'Show your current launcher or game activity in Discord.',
	},
	dataHandling: {
		id: 'app.settings.privacy.data-handling',
		defaultMessage:
			'Telemetry uses a random installation identifier. Error context is sanitized and limited before it leaves this device. Turning telemetry off clears pending reports immediately.',
	},
	logAnalysisTitle: {
		id: 'app.settings.logs.title',
		defaultMessage: 'Log analysis service',
	},
	logAnalysisDescription: {
		id: 'app.settings.logs.description',
		defaultMessage:
			'Choose the service used to analyze and share Minecraft logs. LogShare.CN is preferred and mclo.gs is used as a fallback so sharing keeps working.',
	},
	auto: {
		id: 'app.settings.logs.provider.auto',
		defaultMessage: 'Automatic (recommended)',
	},
	autoDescription: {
		id: 'app.settings.logs.provider.auto-description',
		defaultMessage:
			'Prefers LogShare.CN, automatically switching to mclo.gs when it is unavailable.',
	},
	logshare: {
		id: 'app.settings.logs.provider.logshare',
		defaultMessage: 'LogShare.CN',
	},
	logshareDescription: {
		id: 'app.settings.logs.provider.logshare-description',
		defaultMessage:
			'Uses LogShare.CN for analysis and sharing, including AI analysis, falling back to mclo.gs when unavailable.',
	},
	mclogs: {
		id: 'app.settings.logs.provider.mclogs',
		defaultMessage: 'mclo.gs',
	},
	mclogsDescription: {
		id: 'app.settings.logs.provider.mclogs-description',
		defaultMessage: 'Uses mclo.gs for analysis and sharing. AI analysis is not available.',
	},
})

const providerInfo: Record<LogShareProvider, { label: string; description: string }> = {
	auto: {
		label: formatMessage(messages.auto),
		description: formatMessage(messages.autoDescription),
	},
	logshare: {
		label: formatMessage(messages.logshare),
		description: formatMessage(messages.logshareDescription),
	},
	mclogs: {
		label: formatMessage(messages.mclogs),
		description: formatMessage(messages.mclogsDescription),
	},
}

const options = Object.entries(providerInfo).map(([value, info]) => ({
	value,
	label: info.label,
}))

const selectedInfo = computed(() => providerInfo[selectedProvider.value])

watch(selectedProvider, (provider) => {
	setLogShareProvider(provider)
})

async function updateTelemetry(value: boolean) {
	if (telemetrySaving.value) return
	const previous = privacy.value.telemetry
	privacy.value.telemetry = value
	telemetrySaving.value = true
	try {
		const saved = await setTelemetryEnabled(value)
		privacy.value.telemetry = saved.telemetry
		privacy.value.consent_version = saved.consent_version
	} catch (error) {
		privacy.value.telemetry = previous
		handleError(error)
	} finally {
		telemetrySaving.value = false
	}
}

async function updateDiscordRpc(value: boolean) {
	if (discordSaving.value) return
	const previous = privacy.value.discord_rpc
	privacy.value.discord_rpc = value
	discordSaving.value = true
	try {
		const saved = await setDiscordRpcEnabled(value)
		privacy.value.discord_rpc = saved.discord_rpc
	} catch (error) {
		privacy.value.discord_rpc = previous
		handleError(error)
	} finally {
		discordSaving.value = false
	}
}
</script>

<template>
	<div class="flex max-w-3xl flex-col gap-6">
		<div class="flex items-center justify-between gap-4">
			<div class="flex min-w-0 flex-col gap-1">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.telemetry) }}
				</h3>
				<p class="m-0 leading-tight text-secondary">
					{{ formatMessage(messages.telemetryDescription) }}
				</p>
			</div>
			<Toggle
				id="privacy-telemetry"
				:model-value="privacy.telemetry"
				:disabled="telemetrySaving"
				@update:model-value="(value) => updateTelemetry(!!value)"
			/>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div class="flex min-w-0 flex-col gap-1">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.discordRpc) }}
				</h3>
				<p class="m-0 leading-tight text-secondary">
					{{ formatMessage(messages.discordRpcDescription) }}
				</p>
			</div>
			<Toggle
				id="privacy-discord-rpc"
				:model-value="privacy.discord_rpc"
				:disabled="discordSaving"
				@update:model-value="(value) => updateDiscordRpc(!!value)"
			/>
		</div>

		<p class="m-0 text-sm leading-relaxed text-secondary">
			{{ formatMessage(messages.dataHandling) }}
		</p>

		<div class="grid grid-cols-[minmax(0,1fr)_11rem] items-center gap-6">
			<div class="flex min-w-0 flex-col gap-1">
				<h3 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.logAnalysisTitle) }}
				</h3>
				<p class="m-0 leading-tight text-secondary">
					{{ formatMessage(messages.logAnalysisDescription) }}
				</p>
			</div>
			<div class="w-44">
				<Combobox
					id="log-share-provider"
					v-model="selectedProvider"
					name="Log analysis service"
					:options="options"
				/>
			</div>
		</div>

		<p class="m-0 rounded-xl bg-surface-4 p-4 text-sm leading-tight text-secondary">
			{{ selectedInfo.description }}
		</p>
	</div>
</template>
