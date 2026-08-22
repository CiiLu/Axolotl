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

import SettingsRow from './SettingsRow.vue'
import SettingsSaveStatus from './SettingsSaveStatus.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const privacy = ref(await getPrivacySettings())
const telemetrySaving = ref(false)
const discordSaving = ref(false)
const selectedProvider = ref<LogShareProvider>(getLogShareProvider())
const lastSaveState = ref<'idle' | 'saved' | 'error'>('idle')
const retrySave = ref<(() => void) | undefined>()

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
const saveStatus = computed(() => {
	if (telemetrySaving.value || discordSaving.value) return 'saving'
	return lastSaveState.value
})

watch(selectedProvider, (provider) => {
	setLogShareProvider(provider)
})

async function updateTelemetry(value: boolean) {
	if (telemetrySaving.value) return
	const previous = privacy.value.telemetry
	privacy.value.telemetry = value
	telemetrySaving.value = true
	lastSaveState.value = 'idle'
	retrySave.value = undefined
	try {
		const saved = await setTelemetryEnabled(value)
		privacy.value.telemetry = saved.telemetry
		privacy.value.consent_version = saved.consent_version
		lastSaveState.value = 'saved'
	} catch (error) {
		privacy.value.telemetry = previous
		retrySave.value = () => void updateTelemetry(value)
		lastSaveState.value = 'error'
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
	lastSaveState.value = 'idle'
	retrySave.value = undefined
	try {
		const saved = await setDiscordRpcEnabled(value)
		privacy.value.discord_rpc = saved.discord_rpc
		lastSaveState.value = 'saved'
	} catch (error) {
		privacy.value.discord_rpc = previous
		retrySave.value = () => void updateDiscordRpc(value)
		lastSaveState.value = 'error'
		handleError(error)
	} finally {
		discordSaving.value = false
	}
}
</script>

<template>
	<div class="settings-page">
		<header class="settings-page-header">
			<SettingsSaveStatus :status="saveStatus" :retry="retrySave" />
		</header>
		<div class="settings-page-card">
			<SettingsRow>
				<template #label>
					<span id="settings-target-privacy-telemetry" tabindex="-1">
						{{ formatMessage(messages.telemetry) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.telemetryDescription) }}</template>
				<template #control>
					<Toggle
						id="privacy-telemetry"
						:model-value="privacy.telemetry"
						:disabled="telemetrySaving"
						@update:model-value="(value) => updateTelemetry(!!value)"
					/>
				</template>
			</SettingsRow>
			<SettingsRow>
				<template #label>
					<span id="settings-target-privacy-discord-rpc" tabindex="-1">
						{{ formatMessage(messages.discordRpc) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.discordRpcDescription) }}</template>
				<template #control>
					<Toggle
						id="privacy-discord-rpc"
						:model-value="privacy.discord_rpc"
						:disabled="discordSaving"
						@update:model-value="(value) => updateDiscordRpc(!!value)"
					/>
				</template>
			</SettingsRow>
		</div>

		<p class="settings-page-note">{{ formatMessage(messages.dataHandling) }}</p>
		<div class="settings-page-card">
			<SettingsRow>
				<template #label>
					<span id="settings-target-privacy-log-analysis" tabindex="-1">
						{{ formatMessage(messages.logAnalysisTitle) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.logAnalysisDescription) }}</template>
				<template #control>
					<Combobox
						id="log-share-provider"
						v-model="selectedProvider"
						:name="formatMessage(messages.logAnalysisTitle)"
						:options="options"
					/>
				</template>
			</SettingsRow>
			<p class="settings-page-note settings-page-note-inset">{{ selectedInfo.description }}</p>
		</div>
	</div>
</template>

<style scoped>
.settings-page {
	display: flex;
	width: 100%;
	flex-direction: column;
	gap: var(--gap-lg);
}

.settings-page-card {
	overflow: hidden;
	border: 1px solid var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
}

.settings-page-header {
	display: flex;
	min-height: 0;
	justify-content: flex-end;
}

.settings-page-note {
	margin: 0;
	color: var(--color-secondary);
	font-size: 0.8125rem;
	line-height: 1.5;
}

.settings-page-note-inset {
	margin: 0 var(--gap-lg) var(--gap-lg);
	padding: var(--gap-md);
	border-radius: var(--radius-sm);
	background: var(--surface-3);
}
</style>
