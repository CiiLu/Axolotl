<script setup lang="ts">
import { SaveIcon, SpinnerIcon } from '@modrinth/assets'
import {
	configFieldLabel,
	getConfigFile,
	parseProperties,
	type PropertiesEntry,
	resolveConfigField,
	serializeProperties,
	setProperty,
} from '@modrinth/server'
import {
	ButtonStyled,
	defineMessages,
	DropdownSelect,
	injectNotificationManager,
	StyledInput,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

import { servers } from '@/helpers/servers'

const props = defineProps<{
	serverId: string
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	title: { id: 'app.servers.properties.title', defaultMessage: 'server.properties' },
	formMode: { id: 'app.servers.properties.mode.form', defaultMessage: 'Form' },
	textMode: { id: 'app.servers.properties.mode.text', defaultMessage: 'Text' },
	save: { id: 'app.servers.properties.save', defaultMessage: 'Save changes' },
	saved: { id: 'app.servers.properties.saved', defaultMessage: 'Server settings saved' },
	missing: {
		id: 'app.servers.properties.missing',
		defaultMessage: 'Start the server once to generate this file.',
	},
	loadFailed: {
		id: 'app.servers.properties.load-failed',
		defaultMessage: 'Failed to load the server configuration.',
	},
})

const FILE_NAME = 'server.properties'

const isLoading = ref(true)
const isMissing = ref(false)
const isSaving = ref(false)
const mode = ref<'form' | 'text'>('form')
const entries = ref<PropertiesEntry[]>([])
const rawText = ref('')
const { addNotification, handleError } = injectNotificationManager()

async function load() {
	isLoading.value = true
	isMissing.value = false
	try {
		const text = await servers.readFile(props.serverId, FILE_NAME)
		entries.value = parseProperties(text)
		rawText.value = text
	} catch {
		isMissing.value = true
	} finally {
		isLoading.value = false
	}
}

onMounted(load)

const definition = computed(() => getConfigFile(FILE_NAME))

const formFields = computed(() =>
	entries.value
		.map((entry) => (entry.type === 'pair' ? entry : null))
		.filter((entry): entry is Extract<PropertiesEntry, { type: 'pair' }> => entry !== null)
		.map((pair) => ({
			key: pair.key,
			value: pair.value,
			field: definition.value
				? resolveConfigField(definition.value, pair.key, pair.value)
				: { key: pair.key, kind: 'string' as const, inferred: true },
		})),
)

function setFieldValue(key: string, value: string) {
	entries.value = setProperty(entries.value, key, value)
}

function switchMode(next: 'form' | 'text') {
	if (next === 'text' && mode.value === 'form') {
		rawText.value = serializeProperties(entries.value)
	} else if (next === 'form' && mode.value === 'text') {
		entries.value = parseProperties(rawText.value)
	}
	mode.value = next
}

async function save() {
	isSaving.value = true
	try {
		const text = mode.value === 'text' ? rawText.value : serializeProperties(entries.value)
		await servers.writeFile(props.serverId, FILE_NAME, text)
		entries.value = parseProperties(text)
		rawText.value = text
		addNotification?.({ type: 'success', title: formatMessage(messages.saved) })
	} catch (error) {
		handleError?.(error)
	} finally {
		isSaving.value = false
	}
}
</script>

<template>
	<section data-onboarding-id="server-properties" class="flex flex-col gap-4">
		<div class="flex items-center justify-between gap-3">
			<h3 class="m-0 font-mono text-base font-semibold text-contrast">
				{{ formatMessage(messages.title) }}
			</h3>
			<div class="flex items-center gap-2">
				<ButtonStyled :type="mode === 'form' ? 'highlight' : 'transparent'" size="small">
					<button type="button" @click="switchMode('form')">
						{{ formatMessage(messages.formMode) }}
					</button>
				</ButtonStyled>
				<ButtonStyled :type="mode === 'text' ? 'highlight' : 'transparent'" size="small">
					<button type="button" @click="switchMode('text')">
						{{ formatMessage(messages.textMode) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<p v-if="isMissing" class="m-0 text-secondary">
			{{ formatMessage(messages.missing) }}
		</p>

		<template v-else-if="mode === 'form'">
			<div
				class="grid grid-cols-1 gap-px overflow-hidden rounded-xl border border-solid border-surface-4 bg-surface-4 md:grid-cols-2"
			>
				<template v-for="item in formFields" :key="item.key">
					<div
						v-if="item.field.kind === 'boolean'"
						class="flex min-w-0 items-center justify-between gap-3 bg-surface-2 px-4 py-3"
					>
						<label
							class="truncate text-sm font-medium text-contrast"
							:for="`server-prop-${item.key}`"
						>
							{{ configFieldLabel(item.key) }}
						</label>
						<Toggle
							:id="`server-prop-${item.key}`"
							:model-value="item.value === 'true'"
							small
							@update:model-value="setFieldValue(item.key, $event ? 'true' : 'false')"
						/>
					</div>

					<div
						v-else-if="item.field.kind === 'integer' || item.field.kind === 'number'"
						class="flex min-w-0 items-center justify-between gap-3 bg-surface-2 px-4 py-3"
					>
						<label
							class="truncate text-sm font-medium text-contrast"
							:for="`server-prop-${item.key}`"
						>
							{{ configFieldLabel(item.key) }}
						</label>
						<StyledInput
							:id="`server-prop-${item.key}`"
							:model-value="item.value"
							inputmode="numeric"
							size="small"
							wrapper-class="w-32 shrink-0"
							@update:model-value="setFieldValue(item.key, $event)"
						/>
					</div>

					<div
						v-else-if="item.field.kind === 'enum'"
						class="flex min-w-0 items-center justify-between gap-3 bg-surface-2 px-4 py-3"
					>
						<label
							class="truncate text-sm font-medium text-contrast"
							:for="`server-prop-${item.key}`"
						>
							{{ configFieldLabel(item.key) }}
						</label>
						<DropdownSelect
							:model-value="item.value"
							:options="item.field.options ?? []"
							:name="`server-prop-${item.key}`"
							class="!w-40 shrink-0"
							@update:model-value="setFieldValue(item.key, $event)"
						/>
					</div>

					<div
						v-else
						class="flex min-w-0 items-center justify-between gap-3 bg-surface-2 px-4 py-3"
					>
						<label
							class="truncate text-sm font-medium text-contrast"
							:for="`server-prop-${item.key}`"
						>
							{{ configFieldLabel(item.key) }}
						</label>
						<StyledInput
							:id="`server-prop-${item.key}`"
							:model-value="item.value"
							size="small"
							wrapper-class="w-44 shrink-0"
							@update:model-value="setFieldValue(item.key, $event)"
						/>
					</div>
				</template>
			</div>
		</template>

		<textarea
			v-else
			v-model="rawText"
			class="box-border min-h-72 w-full resize-y rounded-xl border border-solid border-surface-4 bg-surface-3 px-3 py-2 font-mono text-sm text-contrast outline-none transition-colors focus:border-brand"
			spellcheck="false"
		></textarea>

		<div class="flex justify-end">
			<ButtonStyled color="brand">
				<button type="button" :disabled="isSaving || isMissing" @click="save">
					<SpinnerIcon v-if="isSaving" class="animate-spin" />
					<SaveIcon v-else />
					{{ formatMessage(messages.save) }}
				</button>
			</ButtonStyled>
		</div>
	</section>
</template>
