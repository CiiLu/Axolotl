<script setup lang="ts">
import { DownloadIcon, ServerIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	DropdownSelect,
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
	namePlaceholder: {
		id: 'app.servers.wizard.name-placeholder',
		defaultMessage: 'e.g. Survival with friends',
	},
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	javaNone: { id: 'app.servers.settings.java-none', defaultMessage: 'System default' },
	javaMissing: {
		id: 'app.servers.wizard.java-missing',
		defaultMessage: 'No compatible Java installation found.',
	},
	installJava: { id: 'app.servers.wizard.install-java', defaultMessage: 'Install Java' },
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory' },
	memoryValue: { id: 'app.servers.wizard.memory-value', defaultMessage: '{value} MB' },
})

const javaPathOptions = computed(() => ['', ...ctx.javaOptions.value.map((java) => java.path)])

function javaOptionLabel(value: string) {
	if (value === '') return formatMessage(messages.javaNone)
	const java = ctx.javaOptions.value.find((entry) => entry.path === value)
	return java ? 'Java ' + java.version : value
}

onMounted(() => {
	if (ctx.javaOptions.value.length === 0) void ctx.loadJavaOptions()
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
			<DropdownSelect
				v-model="ctx.selectedJavaPath.value"
				:options="javaPathOptions"
				:display-name="javaOptionLabel"
				:name="formatMessage(messages.java)"
			/>
			<div
				v-if="ctx.javaOptions.value.length === 0 && !ctx.isJavaLoading.value"
				class="flex items-center justify-between gap-3"
			>
				<span class="text-sm text-orange">
					{{ formatMessage(messages.javaMissing) }}
				</span>
				<ButtonStyled color="brand" size="small">
					<button type="button" :disabled="ctx.isInstallingJava.value" @click="ctx.installJava()">
						<SpinnerIcon v-if="ctx.isInstallingJava.value" class="animate-spin" />
						<DownloadIcon v-else />
						{{ formatMessage(messages.installJava) }}
					</button>
				</ButtonStyled>
			</div>
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
