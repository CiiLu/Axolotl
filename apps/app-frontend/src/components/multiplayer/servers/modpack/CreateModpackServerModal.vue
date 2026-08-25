<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { commonMessages, MultiStageModal } from '@modrinth/ui'
import { computed, ref, useTemplateRef, watch } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import { provideCreateServerFlow } from '@/components/multiplayer/servers/create-server-flow'
import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import {
	createModpackServerFlowContext,
	provideModpackServerFlow,
} from '@/components/multiplayer/servers/modpack/create-modpack-server-flow'

const emit = defineEmits<{
	created: [serverId: string]
}>()

const modal = useTemplateRef<ComponentExposed<typeof MultiStageModal>>('modal')
const eulaModal = useTemplateRef<ComponentExposed<typeof EulaModal>>('eulaModal')

const ctx = createModpackServerFlowContext(modal)
provideCreateServerFlow(ctx)
provideModpackServerFlow(ctx)

const wasHiddenDuringInstall = ref(false)

const cancelButton = computed(() => ({
	label: ctx.formatMessage(commonMessages.cancelButton),
	disabled: ctx.installPhase.value === 'downloading' || ctx.installPhase.value === 'first-run',
	onClick: () => modal.value?.hide(),
}))

watch(ctx.showEulaModal, (visible) => {
	if (visible) eulaModal.value?.show()
	else eulaModal.value?.hide()
})

// The download keeps running in the background even if the wizard is closed.
// Report the finished server once the flow reaches a terminal success state.
watch(
	() => ctx.installPhase.value,
	(phase) => {
		if (phase === 'done' && wasHiddenDuringInstall.value && ctx.createdServer.value) {
			emit('created', ctx.createdServer.value.id)
		}
	},
)

function show(project: Labrinth.Projects.v2.Project, version: Labrinth.Versions.v2.Version) {
	wasHiddenDuringInstall.value = false
	ctx.reset()
	ctx.setPack(project, version)
	modal.value?.setStage(0)
	modal.value?.show()
}

function handleHide() {
	if (ctx.createdServer.value) emit('created', ctx.createdServer.value.id)
	else wasHiddenDuringInstall.value = true
}

defineExpose({ show, hide: () => modal.value?.hide() })
</script>

<template>
	<MultiStageModal
		ref="modal"
		:stages="ctx.stageConfigs"
		:context="ctx"
		breadcrumbs
		:back-button-enabled="
			(flowCtx) =>
				flowCtx.installPhase.value !== 'downloading' && flowCtx.installPhase.value !== 'first-run'
		"
		:cancel-button="cancelButton"
		@hide="handleHide"
	/>
	<EulaModal
		ref="eulaModal"
		:text="ctx.eulaText.value"
		@accept="ctx.acceptEula"
		@decline="ctx.declineEula"
	/>
</template>
