<script setup lang="ts">
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	ExternalIcon,
	GlobeIcon,
	SparklesIcon,
	XIcon,
} from '@modrinth/assets'
import {
	AutoLink,
	ButtonStyled,
	Collapsible,
	defineMessages,
	NewModal,
	TagItem,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { GcContext } from '@/helpers/gc/types'
import {
	getJavaArgumentPresets,
	getPresetsByGroup,
	type JavaArgumentPreset,
} from '@/helpers/java-argument-presets'

const model = defineModel<string>({ required: true })

const props = withDefaults(
	defineProps<{
		id?: string
		placeholder?: string
		disabled?: boolean
		gcContext?: GcContext
	}>(),
	{
		id: undefined,
		placeholder: undefined,
		disabled: false,
		gcContext: undefined,
	},
)

const { formatMessage } = useVIntl()

const messages = defineMessages({
	presetsButton: {
		id: 'app.java-arguments.presets.button',
		defaultMessage: 'Argument presets',
	},
	presetsModalTitle: {
		id: 'app.java-arguments.presets.modal-title',
		defaultMessage: 'Java argument presets',
	},
	usePreset: {
		id: 'app.java-arguments.presets.use',
		defaultMessage: 'Use preset',
	},
	presetApplied: {
		id: 'app.java-arguments.presets.applied',
		defaultMessage: 'Applied',
	},
	removePreset: {
		id: 'app.java-arguments.presets.remove',
		defaultMessage: 'Remove preset',
	},
	presetArguments: {
		id: 'app.java-arguments.presets.arguments',
		defaultMessage: 'Arguments',
	},
	gcGroupTitle: {
		id: 'app.java-arguments.presets.gc-group-title',
		defaultMessage: 'Memory recycling strategy (GC)',
	},
	autoResolved: {
		id: 'app.java-arguments.presets.gc.auto.resolved',
		defaultMessage: 'Resolved → {strategy}',
	},
	autoReasonChain: {
		id: 'app.java-arguments.presets.gc.auto.reason-chain',
		defaultMessage: 'Decision path: {chain}',
	},
})

const presets = computed(() => getJavaArgumentPresets(props.gcContext))
const groupedPresets = computed(() => getPresetsByGroup(presets.value))

const modal = ref<InstanceType<typeof NewModal>>()
const collapsedPresetIds = ref(new Set<string>())
const copiedPresetId = ref<string | null>(null)

function splitPreset(value: string) {
	const trimmed = value.trimStart()
	for (const preset of presets.value) {
		if (trimmed.startsWith(preset.args)) {
			return { preset, rest: trimmed.slice(preset.args.length).trimStart() }
		}
		if (preset.detect && preset.detect(trimmed)) {
			return { preset, rest: '' }
		}
	}
	return { preset: undefined, rest: value }
}

const activePreset = computed(() => splitPreset(model.value).preset)

const rest = computed<string>({
	get: () => splitPreset(model.value).rest,
	set: (value) => {
		const current = splitPreset(model.value)
		model.value = current.preset ? current.preset.args + (value ? ` ${value}` : '') : value
	},
})

function onInput(event: Event) {
	rest.value = (event.target as HTMLInputElement).value
}

function removeOtherGroupPresets(preset: JavaArgumentPreset, currentArgs: string): string {
	if (!preset.group) return currentArgs

	const groupPresets = presets.value.filter((p) => p.group === preset.group && p.id !== preset.id)
	let result = currentArgs
	for (const other of groupPresets) {
		if (other.detect && other.detect(result)) {
			result = result.replace(other.args, '').trimStart()
		}
	}
	return result
}

function applyPreset(preset: JavaArgumentPreset) {
	const currentRest = splitPreset(model.value).rest
	const argsToApply = preset.resolveArgs ? preset.resolveArgs(props.gcContext) : preset.args
	const cleanedRest = removeOtherGroupPresets(preset, currentRest)
	model.value = argsToApply + (cleanedRest ? ` ${cleanedRest}` : '')
	modal.value?.hide()
}

function removePreset() {
	if (!activePreset.value) return
	model.value = splitPreset(model.value).rest
}

function showPresets() {
	modal.value?.show()
}

async function copyPresetArgs(preset: JavaArgumentPreset) {
	const argsToCopy = preset.resolveArgs ? preset.resolveArgs(props.gcContext) : preset.args
	await navigator.clipboard.writeText(argsToCopy)
	copiedPresetId.value = preset.id
	setTimeout(() => {
		if (copiedPresetId.value === preset.id) {
			copiedPresetId.value = null
		}
	}, 1500)
}

function isPresetCollapsed(preset: JavaArgumentPreset) {
	return collapsedPresetIds.value.has(preset.id)
}

function togglePresetCollapsed(preset: JavaArgumentPreset) {
	const next = new Set(collapsedPresetIds.value)
	if (next.has(preset.id)) {
		next.delete(preset.id)
	} else {
		next.add(preset.id)
	}
	collapsedPresetIds.value = next
}

function isPresetActive(preset: JavaArgumentPreset) {
	return activePreset.value?.id === preset.id
}

function getDisplayArgs(preset: JavaArgumentPreset): string {
	return preset.resolveArgs ? preset.resolveArgs(props.gcContext) : preset.args
}

function getAutoResolvedLabel(preset: JavaArgumentPreset): string | null {
	if (preset.id !== 'gc-auto' || !preset.autoResolvedName) return null
	return formatMessage(messages.autoResolved, { strategy: preset.autoResolvedName })
}

function getAutoReasonChainText(preset: JavaArgumentPreset): string | null {
	if (preset.id !== 'gc-auto' || !preset.autoReasonChain) return null
	return formatMessage(messages.autoReasonChain, { chain: preset.autoReasonChain.join(' → ') })
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<div class="flex items-center gap-2">
			<div
				class="flex min-w-0 flex-1 items-center gap-2 rounded-xl bg-surface-4 px-3 transition-[box-shadow,color] focus-within:ring-4 focus-within:ring-brand-shadow"
				:class="props.disabled ? 'cursor-not-allowed opacity-50' : ''"
			>
				<TagItem
					v-if="activePreset"
					class="shrink-0"
					:action="props.disabled ? undefined : removePreset"
					:aria-label="formatMessage(messages.removePreset)"
				>
					{{ formatMessage(activePreset.title) }}
					<XIcon aria-hidden="true" />
				</TagItem>
				<input
					:id="props.id"
					:value="rest"
					:disabled="props.disabled"
					:placeholder="props.placeholder"
					class="h-9 min-w-0 flex-1 bg-transparent px-0 py-2 text-base font-medium text-primary placeholder:text-secondary focus:text-contrast focus:shadow-none focus:outline-none"
					autocomplete="off"
					type="text"
					@input="onInput"
				/>
			</div>
			<ButtonStyled type="outlined" class="shrink-0">
				<button type="button" :disabled="props.disabled" @click="showPresets">
					<SparklesIcon aria-hidden="true" />
					{{ formatMessage(messages.presetsButton) }}
				</button>
			</ButtonStyled>
		</div>

		<NewModal
			ref="modal"
			:header="formatMessage(messages.presetsModalTitle)"
			width="min(640px, calc(100vw - 2rem))"
			max-width="640px"
		>
			<div class="flex flex-col gap-6">
				<div
					v-for="groupEntry in groupedPresets"
					:key="groupEntry.group ?? 'ungrouped'"
					class="flex flex-col gap-3"
				>
					<h3 v-if="groupEntry.group" class="m-0 text-lg font-semibold text-contrast">
						{{
							groupEntry.group === 'gc' ? formatMessage(messages.gcGroupTitle) : groupEntry.group
						}}
					</h3>
					<div
						v-for="preset in groupEntry.presets"
						:key="preset.id"
						class="flex flex-col gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4"
					>
						<div class="flex items-start gap-3 text-left">
							<GlobeIcon class="mt-0.5 size-6 shrink-0 text-secondary" aria-hidden="true" />
							<div class="min-w-0 flex-1">
								<p class="m-0 text-base font-semibold text-contrast">
									{{ formatMessage(preset.title) }}
								</p>
								<AutoLink
									:to="preset.link"
									target="_blank"
									rel="noreferrer"
									class="inline-flex items-start gap-1 text-sm text-secondary hover:text-brand hover:underline"
								>
									<span class="min-w-0">{{ formatMessage(preset.description) }}</span>
									<ExternalIcon class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
								</AutoLink>
								<p
									v-if="getAutoResolvedLabel(preset)"
									class="m-0 mt-2 text-sm font-medium text-brand"
								>
									{{ getAutoResolvedLabel(preset) }}
								</p>
								<p v-if="getAutoReasonChainText(preset)" class="m-0 mt-1 text-xs text-secondary">
									{{ getAutoReasonChainText(preset) }}
								</p>
							</div>
							<ButtonStyled :type="isPresetActive(preset) ? 'standard' : 'outlined'" color="brand">
								<button
									type="button"
									:disabled="isPresetActive(preset)"
									@click="applyPreset(preset)"
								>
									<CheckIcon v-if="isPresetActive(preset)" aria-hidden="true" />
									{{
										formatMessage(
											isPresetActive(preset) ? messages.presetApplied : messages.usePreset,
										)
									}}
								</button>
							</ButtonStyled>
						</div>
						<div class="flex items-center gap-2">
							<div class="h-px min-w-0 flex-1 bg-surface-4" />
							<button
								v-tooltip="formatMessage(messages.presetArguments)"
								type="button"
								:aria-label="formatMessage(messages.presetArguments)"
								class="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-full border-none bg-transparent text-secondary transition-colors hover:bg-surface-5 hover:text-contrast"
								@click="togglePresetCollapsed(preset)"
							>
								<DropdownIcon
									class="size-4 transition-transform"
									:class="{ 'rotate-180': !isPresetCollapsed(preset) }"
									aria-hidden="true"
								/>
							</button>
						</div>
						<Collapsible :collapsed="isPresetCollapsed(preset)">
							<div class="flex items-start gap-2">
								<code
									class="min-w-0 flex-1 overflow-x-auto whitespace-pre-wrap break-all text-left font-mono text-xs leading-relaxed text-primary"
								>
									{{ getDisplayArgs(preset) }}
								</code>
								<button
									type="button"
									:aria-label="formatMessage(messages.presetArguments)"
									class="flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-full border-none bg-transparent text-secondary transition-colors hover:bg-surface-5 hover:text-contrast"
									@click="copyPresetArgs(preset)"
								>
									<CheckIcon
										v-if="copiedPresetId === preset.id"
										class="size-4 text-green"
										aria-hidden="true"
									/>
									<CopyIcon v-else class="size-4" aria-hidden="true" />
								</button>
							</div>
						</Collapsible>
					</div>
				</div>
			</div>
		</NewModal>
	</div>
</template>
