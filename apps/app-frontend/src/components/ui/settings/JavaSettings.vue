<script setup>
import { ButtonStyled, defineMessages, injectNotificationManager, Toggle, useVIntl } from '@modrinth/ui'
import { platform } from '@tauri-apps/plugin-os'
import { ref, watch } from 'vue'

import JavaSelector from '@/components/ui/JavaSelector.vue'
import { find_filtered_jres, get_java_versions, set_java_version } from '@/helpers/jre'
import { get, set } from '@/helpers/settings.ts'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	javaLocation: {
		id: 'app.settings.java.location',
		defaultMessage: 'Java {version} location',
	},
	autoHighPerformanceMode: {
		id: 'app.settings.java.auto-high-performance-mode',
		defaultMessage: 'Automatically use high-performance GPU for Java',
	},
	autoHighPerformanceModeDescription: {
		id: 'app.settings.java.auto-high-performance-mode-description',
		defaultMessage:
			'Sets the launcher and Java to use the high-performance GPU in Windows graphics settings when Minecraft launches. Windows only.',
	},
	deepScanConfirm: {
		id: 'app.settings.java.deep-scan-confirm',
		defaultMessage: 'This will scan ALL directories on ALL drives. May take several minutes.',
	},
	scanAnyway: {
		id: 'app.settings.java.scan-anyway',
		defaultMessage: 'Scan Anyway',
	},
	cancel: {
		id: 'app.settings.java.cancel',
		defaultMessage: 'Cancel',
	},
})

const javaVersions = ref(await get_java_versions().catch(handleError))
const settings = ref(await get().catch(handleError))
const isWindows = (await platform()) === 'windows'

watch(
	() => settings.value?.auto_set_java_high_performance_mode,
	async () => {
		if (settings.value) {
			await set(settings.value).catch(handleError)
		}
	},
)

const showDeepScanConfirm = ref(false)

async function runScan(deep) {
	if (deep) {
		showDeepScanConfirm.value = true
		return
	}
	const jres = await find_filtered_jres(null, false, true, false).catch(handleError)
	if (jres) {
		javaVersions.value = await get_java_versions().catch(handleError)
	}
}

function confirmDeepScan() {
	showDeepScanConfirm.value = false
	find_filtered_jres(null, true, true, true)
		.then(() => get_java_versions())
		.then((versions) => {
			javaVersions.value = versions
		})
		.catch(handleError)
}

async function updateJavaVersion(version) {
	if (version?.path === '') {
		version.path = undefined
	}

	if (version?.path) {
		version.path = version.path.replace('java.exe', 'javaw.exe')
	}

	await set_java_version(version).catch(handleError)
}
</script>
<template>
	<div class="flex flex-col gap-6">
		<div class="flex items-center gap-2">
			<ButtonStyled @click="runScan(false)">
				{{ 'Find Java' }}
			</ButtonStyled>
			<ButtonStyled type="outlined" @click="runScan(true)">
				{{ 'Deep Scan' }}
			</ButtonStyled>
		</div>
		<div v-if="showDeepScanConfirm" class="flex items-center gap-2 p-2 bg-warning/10 rounded-lg border border-warning text-sm">
			<span>{{ formatMessage(messages.deepScanConfirm) }}</span>
			<ButtonStyled color="red" @click="confirmDeepScan">{{ formatMessage(messages.scanAnyway) }}</ButtonStyled>
			<ButtonStyled type="outlined" @click="showDeepScanConfirm = false">{{ formatMessage(messages.cancel) }}</ButtonStyled>
		</div>

		<div v-if="settings" class="flex items-center justify-between gap-4">
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.autoHighPerformanceMode) }}
				</h2>
				<p class="m-0 leading-tight">
					{{ formatMessage(messages.autoHighPerformanceModeDescription) }}
				</p>
			</div>
			<Toggle
				id="auto-java-high-performance-mode"
				v-model="settings.auto_set_java_high_performance_mode"
				:disabled="!isWindows"
			/>
		</div>

		<div
			v-for="(javaVersion, index) in [25, 21, 17, 8]"
			:key="`java-${javaVersion}`"
			class="flex flex-col gap-2.5"
		>
			<h2 class="m-0 text-lg font-semibold text-contrast" :class="{ 'mt-4': index !== 0 }">
				{{ formatMessage(messages.javaLocation, { version: javaVersion }) }}
			</h2>
			<JavaSelector
				:id="'java-selector-' + javaVersion"
				v-model="javaVersions[javaVersion]"
				:version="javaVersion"
				@update:model-value="updateJavaVersion"
			/>
		</div>
	</div>
</template>
