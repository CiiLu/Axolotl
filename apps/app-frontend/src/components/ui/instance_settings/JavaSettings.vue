<script setup lang="ts">
import {
	Checkbox,
	defineMessages,
	injectNotificationManager,
	Slider,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, readonly, ref, watch } from 'vue'

import JavaArgumentsInput from '@/components/ui/JavaArgumentsInput.vue'
import JavaSelector from '@/components/ui/JavaSelector.vue'
import MemoryAllocationDisplay from '@/components/ui/MemoryAllocationDisplay.vue'
import useMemorySlider from '@/composables/useMemorySlider'
import { collectGcContext } from '@/helpers/gc/context'
import type { GcContext } from '@/helpers/gc/types'
import { edit, get_optimal_jre_key } from '@/helpers/instance'
import { get } from '@/helpers/settings'
import { injectInstanceSettings } from '@/providers/instance-settings'

import type { AppSettings } from '../../../helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	javaInstallation: {
		id: 'instance.settings.tabs.java.java-installation',
		defaultMessage: 'Java installation',
	},
	customJavaInstallation: {
		id: 'instance.settings.tabs.java.custom-java-installation',
		defaultMessage: 'Use a custom Java installation for this instance',
	},
	javaPathPlaceholder: {
		id: 'instance.settings.tabs.java.java-path-placeholder',
		defaultMessage: '/path/to/java',
	},
	javaMemory: {
		id: 'instance.settings.tabs.java.java-memory',
		defaultMessage: 'Memory allocated',
	},
	customMemoryAllocation: {
		id: 'instance.settings.tabs.java.custom-memory-allocation',
		defaultMessage: 'Custom memory allocation',
	},
	automaticMemory: {
		id: 'instance.settings.tabs.java.automatic-memory',
		defaultMessage: 'Automatically allocate memory at launch',
	},
	javaArguments: {
		id: 'instance.settings.tabs.java.java-arguments',
		defaultMessage: 'Java arguments',
	},
	customJavaArguments: {
		id: 'instance.settings.tabs.java.custom-java-arguments',
		defaultMessage: 'Custom Java arguments',
	},
	enterJavaArguments: {
		id: 'instance.settings.tabs.java.enter-java-arguments',
		defaultMessage: 'Enter Java arguments...',
	},
	javaEnvironmentVariables: {
		id: 'instance.settings.tabs.java.environment-variables',
		defaultMessage: 'Environment variables',
	},
	customEnvironmentVariables: {
		id: 'instance.settings.tabs.java.custom-environment-variables',
		defaultMessage: 'Custom environment variables',
	},
	enterEnvironmentVariables: {
		id: 'instance.settings.tabs.java.enter-environment-variables',
		defaultMessage: 'Enter environmental variables...',
	},
})

const { instance } = injectInstanceSettings()

const globalSettings = (await get().catch(handleError)) as unknown as AppSettings
const optimalJava = readonly(await get_optimal_jre_key(instance.value.id).catch(handleError))
const requiredJavaVersion = optimalJava?.parsed_version ?? null

const overrideJavaInstall = ref(!!instance.value.java_path)
const overrideJava = ref({
	...(optimalJava ?? {}),
	path: instance.value.java_path ?? optimalJava?.path ?? '',
})
const displayedJava = computed({
	get: () => (overrideJavaInstall.value ? overrideJava.value : (optimalJava ?? overrideJava.value)),
	set: (value) => {
		overrideJava.value = value
	},
})

watch(overrideJavaInstall, (enabled) => {
	if (enabled && !overrideJava.value.path) {
		overrideJava.value = { ...(optimalJava ?? {}), path: optimalJava?.path ?? '' }
	}
})

const overrideJavaArgs = ref((instance.value.extra_launch_args?.length ?? 0) > 0)
const javaArgs = ref(
	(instance.value.extra_launch_args ?? globalSettings?.extra_launch_args ?? []).join(' '),
)

const overrideEnvVars = ref((instance.value.custom_env_vars?.length ?? 0) > 0)
const envVars = ref(
	(instance.value.custom_env_vars ?? globalSettings?.custom_env_vars ?? [])
		.map((x: string[]) => x.join('='))
		.join(' '),
)

const overrideMemorySettings = ref(!!instance.value.memory)
const memory = ref(
	instance.value.memory ?? globalSettings?.memory ?? { maximum: 2048, automatic: true },
)
const effectiveMemory = computed(() =>
	overrideMemorySettings.value
		? memory.value
		: (globalSettings?.memory ?? { maximum: 2048, automatic: true }),
)
const memData = await useMemorySlider().catch(() => ({
	maxMemory: ref(4096),
	snapPoints: computed(() => []),
}))
const maxMemory = memData.maxMemory
const snapPoints = memData.snapPoints

const gcContext = ref<GcContext | null>(null)

async function updateGcContext() {
	gcContext.value = await collectGcContext(memory.value.maximum, instance.value.loader)
}

await updateGcContext()

watch([memory, displayedJava], updateGcContext)

const editInstanceObject = computed(() => ({
	java_path:
		overrideJavaInstall.value && overrideJava.value.path
			? overrideJava.value.path.replace('java.exe', 'javaw.exe')
			: null,
	extra_launch_args: overrideJavaArgs.value
		? javaArgs.value.trim().split(/\s+/).filter(Boolean)
		: null,
	custom_env_vars: overrideEnvVars.value
		? envVars.value
				.trim()
				.split(/\s+/)
				.filter(Boolean)
				.map((x: string) => x.split('=').filter(Boolean))
		: null,
	memory: overrideMemorySettings.value ? memory.value : null,
}))

watch(
	[
		overrideJavaInstall,
		overrideJava,
		overrideJavaArgs,
		javaArgs,
		overrideEnvVars,
		envVars,
		overrideMemorySettings,
		memory,
	],
	async () => {
		await edit(instance.value.id, editInstanceObject.value).catch(handleError)
	},
	{ deep: true },
)
</script>

<template>
	<div>
		<h2 class="m-0 mb-2 block text-base font-extrabold text-contrast">
			{{ formatMessage(messages.javaInstallation) }}
		</h2>
		<Checkbox
			v-model="overrideJavaInstall"
			:label="formatMessage(messages.customJavaInstallation)"
			class="mb-2"
		/>
		<JavaSelector
			v-model="displayedJava"
			:disabled="!overrideJavaInstall"
			:placeholder="formatMessage(messages.javaPathPlaceholder)"
			:version="requiredJavaVersion"
			select-all-versions
		/>

		<h2 class="mb-1 mt-4 block text-base font-extrabold text-contrast">
			{{ formatMessage(messages.javaMemory) }}
		</h2>
		<Checkbox
			v-model="overrideMemorySettings"
			:label="formatMessage(messages.customMemoryAllocation)"
			class="mb-2"
		/>
		<Checkbox
			v-if="overrideMemorySettings"
			v-model="memory.automatic"
			:label="formatMessage(messages.automaticMemory)"
			class="mb-2"
		/>
		<Slider
			id="max-memory"
			v-model="memory.maximum"
			:disabled="!overrideMemorySettings || memory.automatic"
			:min="512"
			:max="maxMemory"
			:step="64"
			:snap-points="snapPoints"
			:snap-range="512"
			unit="MB"
		/>
		<MemoryAllocationDisplay :instance-id="instance.id" :memory="effectiveMemory" />

		<h2 class="mb-1 mt-4 block text-base font-extrabold text-contrast">
			{{ formatMessage(messages.javaArguments) }}
		</h2>
		<Checkbox
			v-model="overrideJavaArgs"
			:label="formatMessage(messages.customJavaArguments)"
			class="my-1"
		/>
		<JavaArgumentsInput
			id="java-args"
			v-model="javaArgs"
			:disabled="!overrideJavaArgs"
			:gc-context="gcContext"
			:placeholder="formatMessage(messages.enterJavaArguments)"
		/>

		<h2 class="mb-1 mt-4 block text-base font-extrabold text-contrast">
			{{ formatMessage(messages.javaEnvironmentVariables) }}
		</h2>
		<Checkbox
			v-model="overrideEnvVars"
			:label="formatMessage(messages.customEnvironmentVariables)"
			class="mb-2"
		/>
		<StyledInput
			id="env-vars"
			v-model="envVars"
			autocomplete="off"
			:disabled="!overrideEnvVars"
			:placeholder="formatMessage(messages.enterEnvironmentVariables)"
			wrapper-class="w-full"
		/>
	</div>
</template>
