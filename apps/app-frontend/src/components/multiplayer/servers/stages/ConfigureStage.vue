<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import ServerPropertiesEditor from '@/components/multiplayer/servers/ServerPropertiesEditor.vue'

import { injectCreateServerFlow } from '../create-server-flow'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	heading: {
		id: 'app.servers.wizard.configure-heading',
		defaultMessage: 'Adjust the server settings, or finish to edit them later.',
	},
})

const serverId = computed(() => ctx.createdServer.value?.id ?? '')
</script>

<template>
	<div class="flex flex-col gap-4">
		<p class="m-0 text-secondary">
			{{ formatMessage(messages.heading) }}
		</p>

		<div class="max-h-[28rem] overflow-y-auto pr-1">
			<ServerPropertiesEditor v-if="serverId !== ''" :server-id="serverId" />
		</div>
	</div>
</template>
