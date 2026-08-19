<script setup lang="ts">
import { MultiStageModal } from '@modrinth/ui'
import { useTemplateRef, watch } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import {
	createCreateServerFlowContext,
	provideCreateServerFlow,
} from '@/components/multiplayer/servers/create-server-flow'
import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'

const emit = defineEmits<{
	created: [serverId: string]
}>()

const modal = useTemplateRef<ComponentExposed<typeof MultiStageModal>>('modal')
const eulaModal = useTemplateRef<ComponentExposed<typeof EulaModal>>('eulaModal')

const ctx = createCreateServerFlowContext(modal)
provideCreateServerFlow(ctx)

watch(ctx.showEulaModal, (visible) => {
	if (visible) eulaModal.value?.show()
	else eulaModal.value?.hide()
})

function show(event?: MouseEvent) {
	ctx.reset()
	modal.value?.show(event)
}

function handleHide() {
	if (ctx.createdServer.value) emit('created', ctx.createdServer.value.id)
}

defineExpose({ show, hide: () => modal.value?.hide() })
</script>

<template>
	<MultiStageModal ref="modal" :stages="ctx.stageConfigs" :context="ctx" @hide="handleHide" />
	<EulaModal
		ref="eulaModal"
		:text="ctx.eulaText.value"
		@accept="ctx.acceptEula"
		@decline="ctx.declineEula"
	/>
</template>
