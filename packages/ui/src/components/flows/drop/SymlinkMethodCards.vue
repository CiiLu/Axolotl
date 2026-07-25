<template>
	<NewModal ref="modal" max-width="480px" :closable="true" hide-header @hide="emit('cancel')">
		<div class="flex flex-col gap-4 p-6">
			<!-- Title -->
			<div class="flex flex-col gap-1">
				<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
				<span class="text-sm text-secondary">
					{{ formatMessage(messages.subtitle, { n: internalInstanceNames.length }) }}
				</span>
			</div>

			<div class="h-px bg-surface-5" />

			<!-- Copy card -->
			<button
				class="group flex flex-col rounded-xl border-2 p-4 transition-all duration-300 cursor-pointer text-left"
				:class="selected === 'copy' ? 'border-brand bg-brand-highlight' : 'border-surface-4 bg-surface-2 hover:border-surface-5'"
				@click="select('copy')"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex size-10 shrink-0 items-center justify-center rounded-lg border border-surface-5 bg-surface-3"
					>
						<CopyIcon class="size-5 text-secondary" stroke-width="1.5" />
					</div>
					<div class="flex flex-col min-w-0 flex-1">
						<span class="text-sm font-semibold text-contrast">{{ formatMessage(messages.copyTitle) }}</span>
						<span class="text-xs text-secondary">{{ formatMessage(messages.copyDesc) }}</span>
					</div>
					<CheckIcon
						v-if="selected === 'copy'"
						class="size-5 text-brand shrink-0"
						stroke-width="2.5"
					/>
				</div>
				<!-- Expandable detail on hover/select -->
				<div
					class="overflow-hidden transition-all duration-300"
					:class="selected === 'copy' ? 'max-h-20 mt-3 opacity-100' : 'max-h-0 group-hover:max-h-20 group-hover:mt-3 group-hover:opacity-100 opacity-0'"
				>
					<p class="text-xs text-secondary m-0">{{ formatMessage(messages.copyDetail) }}</p>
				</div>
			</button>

			<!-- Symlink card -->
			<button
				class="group flex flex-col rounded-xl border-2 p-4 transition-all duration-300 cursor-pointer text-left"
				:class="[
					selected === 'symlink' ? 'border-brand bg-brand-highlight' : 'border-surface-4 bg-surface-2 hover:border-surface-5',
					!symlinkAllowed ? 'opacity-60 pointer-events-none' : '',
				]"
				@click="select('symlink')"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex size-10 shrink-0 items-center justify-center rounded-lg border border-surface-5 bg-surface-3"
					>
						<LinkIcon class="size-5 text-secondary" stroke-width="1.5" />
					</div>
					<div class="flex flex-col min-w-0 flex-1">
						<span class="text-sm font-semibold text-contrast">{{ formatMessage(messages.symlinkTitle) }}</span>
						<span class="text-xs text-secondary">{{ formatMessage(messages.symlinkDesc) }}</span>
					</div>
					<CheckIcon
						v-if="selected === 'symlink'"
						class="size-5 text-brand shrink-0"
						stroke-width="2.5"
					/>
				</div>
				<!-- Expandable detail on hover/select -->
				<div
					class="overflow-hidden transition-all duration-300"
					:class="selected === 'symlink' ? 'max-h-24 mt-3 opacity-100' : 'max-h-0 group-hover:max-h-24 group-hover:mt-3 group-hover:opacity-100 opacity-0'"
				>
					<p class="text-xs text-secondary m-0">{{ formatMessage(messages.symlinkDetail) }}</p>
					<!-- Warning for requires_admin -->
					<span
						v-if="internalSymlinkCapable === 'requires_admin'"
						class="text-xs text-warning mt-1 block"
					>
						{{ formatMessage(messages.requiresAdmin) }}
					</span>
					<span
						v-else-if="internalSymlinkCapable === 'unsupported'"
						class="text-xs text-danger mt-1 block"
					>
						{{ formatMessage(messages.unsupportedWarning) }}
					</span>
				</div>
			</button>
		</div>

		<template #actions>
			<div class="flex w-full items-center justify-between p-4 pt-0">
				<ButtonStyled type="transparent" @click="emit('cancel')">
					{{ formatMessage(messages.cancel) }}
				</ButtonStyled>
				<ButtonStyled :disabled="!selected">
					<button class="flex items-center gap-2" @click="handleConfirm">
						{{ formatMessage(messages.confirm) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, CopyIcon, LinkIcon } from '@modrinth/assets'
import type { PropType } from 'vue'
import { computed, ref } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import NewModal from '#ui/components/modal/NewModal.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'drop.symlink_method.title',
		defaultMessage: 'Choose import method',
	},
	subtitle: {
		id: 'drop.symlink_method.subtitle',
		defaultMessage: 'Importing {n} instance(s)',
	},
	copyTitle: {
		id: 'drop.symlink_method.copy_title',
		defaultMessage: 'Copy files',
	},
	copyDesc: {
		id: 'drop.symlink_method.copy_desc',
		defaultMessage: 'Copy to Axolotl directory',
	},
	copyDetail: {
		id: 'drop.symlink_method.copy_detail',
		defaultMessage: 'Instance files will be copied to Axolotl\'s data directory. This is the default option with the best compatibility.',
	},
	symlinkTitle: {
		id: 'drop.symlink_method.symlink_title',
		defaultMessage: 'Symbolic link',
	},
	symlinkDesc: {
		id: 'drop.symlink_method.symlink_desc',
		defaultMessage: 'Reference original location',
	},
	symlinkDetail: {
		id: 'drop.symlink_method.symlink_detail',
		defaultMessage: 'Instance files stay in their original location. Axolotl references them via a symbolic link. Saves disk space.',
	},
	requiresAdmin: {
		id: 'drop.symlink_method.requires_admin',
		defaultMessage: 'Administrator permission required',
	},
	unsupportedWarning: {
		id: 'drop.symlink_method.unsupported_warning',
		defaultMessage: 'Symbolic links are not supported on this system',
	},
	cancel: {
		id: 'drop.symlink_method.cancel',
		defaultMessage: 'Cancel',
	},
	confirm: {
		id: 'drop.symlink_method.confirm',
		defaultMessage: 'Confirm',
	},
})

const props = defineProps({
	instanceNames: {
		type: Array<string>,
		default: () => [],
	},
	symlinkCapable: {
		type: String as PropType<'supported' | 'requires_admin' | 'unsupported'>,
		default: 'supported',
	},
})

const emit = defineEmits<{
	(e: 'confirm', symlink: boolean): void
	(e: 'cancel'): void
}>()

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const selected = ref<'copy' | 'symlink' | null>(null)
const internalInstanceNames = ref<string[]>([])
const internalSymlinkCapable = ref<'supported' | 'requires_admin' | 'unsupported'>('supported')

const symlinkAllowed = computed(() => {
	return internalSymlinkCapable.value !== 'unsupported'
})

function select(value: 'copy' | 'symlink') {
	selected.value = value
}

function handleConfirm() {
	if (!selected.value) return
	modal.value?.hide()
	emit('confirm', selected.value === 'symlink')
}

function show(options: { instanceNames: string[]; symlinkCapable: 'supported' | 'requires_admin' | 'unsupported' }) {
	internalInstanceNames.value = options.instanceNames
	internalSymlinkCapable.value = options.symlinkCapable
	selected.value = null
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({ show, hide })
</script>
