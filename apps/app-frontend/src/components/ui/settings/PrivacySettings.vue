<script setup lang="ts">
import { defineMessages, injectNotificationManager, Toggle, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { getPrivacySettings, setDiscordRpcEnabled, setTelemetryEnabled } from '@/helpers/settings'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const privacy = ref(await getPrivacySettings())
const telemetrySaving = ref(false)
const discordSaving = ref(false)

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
	<div class="flex max-w-3xl flex-col gap-0">
		<div
			class="flex items-center justify-between gap-6 border-0 border-b border-solid border-surface-4 py-5 first:pt-0"
		>
			<div class="min-w-0">
				<label for="privacy-telemetry" class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.telemetry) }}
				</label>
				<p class="mb-0 mt-1 leading-relaxed text-secondary">
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

		<div class="flex items-center justify-between gap-6 py-5">
			<div class="min-w-0">
				<label for="privacy-discord-rpc" class="text-lg font-semibold text-contrast">
					{{ formatMessage(messages.discordRpc) }}
				</label>
				<p class="mb-0 mt-1 leading-relaxed text-secondary">
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

		<p
			class="m-0 border-0 border-t border-solid border-surface-4 pt-5 text-sm leading-relaxed text-secondary"
		>
			{{ formatMessage(messages.dataHandling) }}
		</p>
	</div>
</template>
