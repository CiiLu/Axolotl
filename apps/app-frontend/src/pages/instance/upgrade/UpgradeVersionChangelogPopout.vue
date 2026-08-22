<template>
	<span
		class="relative inline-flex max-w-full"
		tabindex="0"
		@mouseenter="open"
		@mouseleave="close"
		@focus="open"
		@blur="close"
	>
		<span class="cursor-help underline decoration-dotted underline-offset-2">{{ label }}</span>
		<span
			v-if="visible"
			class="absolute bottom-full left-0 z-30 mb-2 w-80 max-w-[calc(100vw-2rem)] rounded-lg border border-solid border-surface-5 bg-surface-3 p-3 text-left shadow-xl"
			@mouseenter="cancelClose"
			@mouseleave="close"
		>
			<span v-if="loading" class="text-sm text-secondary">{{ formatMessage(messages.loading) }}</span>
			<template v-else-if="metadata">
				<strong class="block text-sm text-contrast">{{ metadata.version }}</strong>
				<span v-if="metadata.channel" class="mt-1 block text-xs text-secondary">{{ metadata.channel }}</span>
				<span v-if="metadata.changelog" class="mt-2 block max-h-40 overflow-auto whitespace-pre-wrap text-sm text-secondary">{{ metadata.changelog }}</span>
				<span v-else class="mt-2 block text-sm text-secondary">{{ formatMessage(messages.empty) }}</span>
			</template>
			<span v-else class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
		</span>
	</span>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { loadUpgradeVersionMetadata } from '@/helpers/upgrade-version-metadata'

const props = defineProps<{ label: string; provider: string | null; projectId: string | null; releaseId: string | null }>()
const messages = defineMessages({
	loading: { id: 'instance.upgrade.changelog.loading', defaultMessage: 'Loading release details…' },
	empty: { id: 'instance.upgrade.changelog.empty', defaultMessage: 'No changelog was provided for this version.' },
	unavailable: { id: 'instance.upgrade.changelog.unavailable', defaultMessage: 'Release details unavailable.' },
})
const { formatMessage } = useVIntl()
const visible = ref(false)
const loading = ref(false)
const metadata = ref<Awaited<ReturnType<typeof loadUpgradeVersionMetadata>> | null>(null)
let closeTimer: ReturnType<typeof setTimeout> | undefined
let loaded = false
function cancelClose() {
	if (closeTimer) clearTimeout(closeTimer)
}
async function open() {
	cancelClose()
	visible.value = true
	if (loaded || !props.provider || !props.projectId || !props.releaseId) return
	loaded = true
	loading.value = true
	try {
		metadata.value = await loadUpgradeVersionMetadata(props.provider, props.projectId, props.releaseId)
	} finally {
		loading.value = false
	}
}
function close() {
	cancelClose()
	closeTimer = setTimeout(() => (visible.value = false), 160)
}
</script>
