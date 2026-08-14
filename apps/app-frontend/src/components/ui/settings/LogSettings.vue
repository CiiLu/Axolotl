<script setup lang="ts">
import {
	Combobox,
	defineMessages,
	getLogShareProvider,
	type LogShareProvider,
	setLogShareProvider,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

const { formatMessage } = useVIntl()
const selectedProvider = ref<LogShareProvider>(getLogShareProvider())

const messages = defineMessages({
	title: {
		id: 'app.settings.logs.title',
		defaultMessage: 'Log analysis service',
	},
	description: {
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
</script>

<template>
	<div class="flex flex-col gap-6">
		<div class="grid grid-cols-[minmax(0,1fr)_11rem] items-center gap-6">
			<div class="flex min-w-0 flex-col gap-1">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 leading-relaxed text-secondary">
					{{ formatMessage(messages.description) }}
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
