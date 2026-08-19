<script setup lang="ts">
import { ServerIcon } from '@modrinth/assets'
import {
	Combobox,
	type ComboboxOption,
	defineMessages,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted } from 'vue'

import { injectCreateServerFlow } from '../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	name: { id: 'app.servers.wizard.name', defaultMessage: 'Server name' },
	namePlaceholder: { id: 'app.servers.wizard.name-placeholder', defaultMessage: 'Survival' },
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	javaPlaceholder: {
		id: 'app.servers.wizard.java-placeholder',
		defaultMessage: 'Select a Java version',
	},
	javaMissing: {
		id: 'app.servers.wizard.java-missing',
		defaultMessage: 'No Java installations found',
	},
	javaMissingHint: {
		id: 'app.servers.wizard.java-missing-hint',
		defaultMessage:
			'No compatible Java found on your system. The required Java will be installed automatically in the next step.',
	},
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory' },
	memoryValue: { id: 'app.servers.wizard.memory-value', defaultMessage: '{value} MB' },
})

const javaOptions = computed<ComboboxOption<string>[]>(() =>
	ctx.javaOptions.value.map((java) => ({
		value: java.path,
		label: `Java ${java.version}`,
		subLabel: java.path,
	})),
)

const selectedJava = computed({
	get: () => ctx.selectedJavaPath.value,
	set: (value) => {
		if (typeof value === 'string' && value !== '') ctx.selectedJavaPath.value = value
	},
})

function suggestName() {
	const type = ctx.serverType.value
	const version = ctx.selectedGameVersion.value
	const flag = Math.random().toString(16).slice(2, 6)
	const segments = [type, version]
	if (ctx.selectedLoaderVersion.value) segments.push(ctx.selectedLoaderVersion.value)
	segments.push(flag)
	ctx.name.value = segments.filter(Boolean).join('-')
}

onMounted(() => {
	void ctx.loadJavaOptions()
	if (!ctx.name.value.trim() && ctx.selectedGameVersion.value) {
		suggestName()
	}
})
</script>

<template>
	<div class="flex flex-col gap-5">
		<label class="flex min-w-0 flex-col gap-2" for="wizard-server-name">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.name) }}</span>
			<StyledInput
				id="wizard-server-name"
				v-model="ctx.name.value"
				:icon="ServerIcon"
				:placeholder="formatMessage(messages.namePlaceholder)"
			/>
		</label>

		<div class="flex min-w-0 flex-col gap-2">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.java) }}</span>
			<Combobox
				v-model="selectedJava"
				:options="javaOptions"
				:placeholder="formatMessage(messages.javaPlaceholder)"
				:no-options-message="formatMessage(messages.javaMissing)"
				:show-no-options-when-empty="!ctx.isJavaLoading.value"
			/>
			<p
				v-if="ctx.javaOptions.value.length === 0 && !ctx.isJavaLoading.value"
				class="m-0 text-sm text-secondary"
			>
				{{ formatMessage(messages.javaMissingHint) }}
			</p>
		</div>

		<div class="flex min-w-0 flex-col gap-2">
			<div class="flex items-center justify-between gap-3">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.memory) }}</span>
				<span
					class="rounded-md border border-solid border-surface-5 bg-surface-3 px-2 py-1 text-xs font-semibold leading-none text-contrast"
				>
					{{ formatMessage(messages.memoryValue, { value: ctx.memoryMb.value }) }}
				</span>
			</div>
			<Slider v-model="ctx.memoryMb.value" :min="1024" :max="ctx.maxMemoryMb.value" :step="512" />
		</div>
	</div>
</template>
