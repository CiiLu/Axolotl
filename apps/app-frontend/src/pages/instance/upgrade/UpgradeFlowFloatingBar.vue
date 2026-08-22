<template>
	<FloatingActionBar
		:shown="true"
		:aria-label="formatMessage(messages.aria)"
		hide-when-modal-open
		allow-overflow
	>
		<div class="flex min-w-0 items-center gap-3">
			<ButtonStyled type="outlined" size="small">
				<button :disabled="!controls" @click="controls?.onBack()">
					<ArrowLeftIcon aria-hidden="true" />
					<span class="bar-label">{{ formatMessage(messages.back) }}</span>
				</button>
			</ButtonStyled>
			<nav class="hidden min-w-0 select-none items-center gap-1 sm:flex" :aria-label="formatMessage(messages.steps)">
				<template v-for="(step, index) in steps" :key="step.path">
					<span class="flex select-none items-center gap-1" :class="stepClass(index)" :title="formatMessage(step.label)">
						<CheckCircleIcon v-if="index < currentIndex" class="size-4 shrink-0" aria-hidden="true" />
						<span v-else class="size-3 shrink-0 rounded-full border-2 border-current" :class="{ 'bg-current': index === currentIndex }" aria-hidden="true" />
						<span class="bar-label">{{ formatMessage(step.label) }}</span>
					</span>
					<span v-if="index < steps.length - 1" class="mx-1 h-px min-w-3 flex-1 bg-surface-5" aria-hidden="true" />
				</template>
			</nav>
			<span class="sm:hidden text-sm text-secondary">{{ currentIndex + 1 }} / {{ steps.length }}</span>
			<ButtonStyled color="brand" size="small">
				<button :disabled="!controls || !canNext || busy" @click="controls?.onNext()">
					<SpinnerIcon v-if="busy" class="animate-spin" aria-hidden="true" />
					<CircleArrowRightIcon v-else aria-hidden="true" />
					<span class="bar-label">{{ controls?.nextLabel ?? formatMessage(messages.next) }}</span>
				</button>
			</ButtonStyled>
		</div>
	</FloatingActionBar>
</template>

<script setup lang="ts">
import { ArrowLeftIcon, CheckCircleIcon, CircleArrowRightIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, FloatingActionBar, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { upgradeControlEnabled } from './flow-controls'
import { useInstanceUpgradeFlow } from './flow'

const messages = defineMessages({
	aria: { id: 'instance.upgrade.flow.aria', defaultMessage: 'Instance upgrade navigation' },
	back: { id: 'instance.upgrade.flow.back', defaultMessage: 'Previous' },
	next: { id: 'instance.upgrade.flow.next', defaultMessage: 'Next' },
	steps: { id: 'instance.upgrade.flow.steps', defaultMessage: 'Upgrade steps' },
	target: { id: 'instance.upgrade.flow.target', defaultMessage: 'Upgrade target' },
	issues: { id: 'instance.upgrade.flow.issues', defaultMessage: 'Resolve issues' },
	preferences: { id: 'instance.upgrade.flow.preferences', defaultMessage: 'Preferences' },
	confirm: { id: 'instance.upgrade.flow.confirm', defaultMessage: 'Confirm upgrade' },
	progress: { id: 'instance.upgrade.flow.progress', defaultMessage: 'Upgrading' },
	complete: { id: 'instance.upgrade.flow.complete', defaultMessage: 'Complete' },
})
const flow = useInstanceUpgradeFlow()
const route = useRoute()
const { formatMessage } = useVIntl()
const steps = [
	{ path: 'upgrade', label: messages.target },
	{ path: 'upgrade/compatibility', label: messages.issues },
	{ path: 'upgrade/customize', label: messages.preferences },
	{ path: 'upgrade/confirm', label: messages.confirm },
	{ path: 'upgrade/progress', label: messages.progress },
	{ path: 'upgrade/result', label: messages.complete },
]
const currentIndex = computed(() => {
	const path = route.path.replace(`/instance/${encodeURIComponent(flow.instanceId.value)}/`, '')
	const index = steps.findIndex((step) => path.endsWith(step.path))
	return index < 0 ? 0 : index
})
const controls = computed(() => flow.controls.value)
const canNext = computed(() => upgradeControlEnabled(flow.controls.value?.canNext))
const busy = computed(() => upgradeControlEnabled(flow.controls.value?.busy))
function stepClass(index: number) {
	return index < currentIndex.value ? 'text-green' : index === currentIndex.value ? 'font-semibold text-brand' : 'text-secondary'
}
</script>
