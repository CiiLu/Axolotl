<script setup lang="ts">
import { EyeIcon, RefreshCwIcon } from '@modrinth/assets'
import {
	Combobox,
	defineMessages,
	injectNotificationManager,
	NewButton as Button,
	useVIntl,
} from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { inject, ref, watch } from 'vue'

import UpdateAnnouncementHistory from '@/components/ui/announcement/UpdateAnnouncementHistory.vue'
import { getUpdateChannel, setUpdateChannel, type UpdateChannel } from '@/helpers/settings.ts'
import { isDev } from '@/helpers/utils.js'
import { type AppUpdateCheckResult, checkForAppUpdate } from '@/providers/app-update.ts'

import SettingsRow from './SettingsRow.vue'
import SettingsSection from './SettingsSection.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const selectedChannel = ref<UpdateChannel>(await getUpdateChannel())
const checking = ref(false)
const checkResult = ref<AppUpdateCheckResult | 'failed' | 'portable' | null>(null)
const currentVersion = await getVersion()
const isDevEnvironment = await isDev()
const previewUpdateAnnouncement = inject<(version: string) => void>('previewUpdateAnnouncement')
const isPortable = ref(false)

try {
	isPortable.value = await invoke('is_portable_mode')
} catch {
	// Best-effort check: fall back to non-portable when the command is unavailable.
}

const messages = defineMessages({
		title: {
			id: 'app.settings.updates.channel.title',
			defaultMessage: 'Update channel',
		},
		description: {
			id: 'app.settings.updates.channel.description',
			defaultMessage: 'Choose which launcher versions Axolotl receives.',
		},
		release: {
			id: 'app.settings.updates.channel.release',
			defaultMessage: 'Release',
		},
		beta: {
			id: 'app.settings.updates.channel.beta',
			defaultMessage: 'Beta',
	},
	check: {
		id: 'app.settings.updates.check',
		defaultMessage: 'Check for updates',
	},
	checking: {
		id: 'app.settings.updates.checking',
		defaultMessage: 'Checking for updates…',
	},
	available: {
		id: 'app.settings.updates.available',
		defaultMessage: 'An update is available.',
	},
	upToDate: {
		id: 'app.settings.updates.up-to-date',
		defaultMessage: 'Axolotl is up to date.',
	},
	disabled: {
		id: 'app.settings.updates.disabled',
		defaultMessage: 'Updates are disabled in this build.',
	},
	offline: {
		id: 'app.settings.updates.offline',
		defaultMessage: 'Connect to the internet to check for updates.',
	},
	failed: {
		id: 'app.settings.updates.failed',
		defaultMessage: 'Could not check for updates.',
	},
	portable: {
		id: 'app.settings.updates.portable',
		defaultMessage:
			'Portable mode cannot update automatically. Please download the latest version manually.',
	},
	security: {
		id: 'app.settings.updates.security',
		defaultMessage: 'Updates are installed only when their cryptographic signature is valid.',
	},
	preview: {
		id: 'app.settings.updates.preview-announcement',
		defaultMessage: 'Preview update announcement',
	},
})

const options: Array<{ value: UpdateChannel; label: string }> = [
	{ value: 'release', label: formatMessage(messages.release) },
	{ value: 'beta', label: formatMessage(messages.beta) },
]

const resultMessages: Record<AppUpdateCheckResult | 'failed' | 'portable', keyof typeof messages> =
	{
		available: 'available',
		'up-to-date': 'upToDate',
		disabled: 'disabled',
		offline: 'offline',
		failed: 'failed',
		portable: 'portable',
	}

watch(selectedChannel, async (channel) => {
	await setUpdateChannel(channel)
	await invoke('restart_app')
	checkResult.value = null
})

async function checkForUpdates() {
	checking.value = true
	checkResult.value = null

	if (isPortable.value) {
		checkResult.value = 'portable'
		checking.value = false
		return
	}

	try {
		checkResult.value = await checkForAppUpdate()
	} catch (error) {
		checkResult.value = 'failed'
		handleError(error)
	} finally {
		checking.value = false
	}
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<SettingsSection>
			<SettingsRow>
				<template #label>
					<span id="settings-target-updates-channel" tabindex="-1">
						{{ formatMessage(messages.title) }}
					</span>
				</template>
				<template #description>{{ formatMessage(messages.description) }}</template>
				<template #control>
					<Combobox
						id="update-channel"
						v-model="selectedChannel"
						:name="formatMessage(messages.title)"
						:options="options"
					/>
				</template>
			</SettingsRow>
		</SettingsSection>

		<SettingsSection>
			<div class="flex flex-col items-start gap-3 p-4">
				<div class="flex flex-wrap gap-2">
					<Button type="colored" color="brand" :disabled="checking" @click="checkForUpdates">
						<RefreshCwIcon :class="{ 'animate-spin': checking }" />
						{{ formatMessage(checking ? messages.checking : messages.check) }}
					</Button>
					<Button
						v-if="isDevEnvironment && previewUpdateAnnouncement"
						type="outlined"
						native-type="button"
						@click="previewUpdateAnnouncement(currentVersion)"
					>
						<EyeIcon />
						{{ formatMessage(messages.preview) }}
					</Button>
				</div>
				<p v-if="checkResult" class="m-0 text-sm text-secondary" role="status">
					{{ formatMessage(messages[resultMessages[checkResult]]) }}
				</p>
			</div>
		</SettingsSection>

		<p class="settings-note">{{ formatMessage(messages.security) }}</p>

		<UpdateAnnouncementHistory :current-version="currentVersion" />
	</div>
</template>

<style scoped>
.settings-note {
	margin: 0;
	padding: var(--gap-md) var(--gap-lg);
	border: 1px solid
		var(--settings-card-border, color-mix(in srgb, var(--surface-4) 72%, transparent));
	border-radius: var(--radius-md);
	background: var(--surface-2);
	color: var(--color-secondary);
	font-size: 0.8125rem;
	line-height: 1.5;
}
</style>
