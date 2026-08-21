<script setup lang="ts">
import { PlayIcon, StopCircleIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, InstanceRowCard, TagItem, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import {
	isServerStatusVisible,
	SERVER_STATUS_META,
} from '@/components/multiplayer/servers/server-status'
import ServerIcon from '@/components/multiplayer/servers/ServerIcon.vue'
import type { ServerView } from '@/composables/useServers'

const props = defineProps<{
	server: ServerView
}>()

const emit = defineEmits<{
	open: []
	'start-stop': []
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
})

const statusMeta = computed(() => SERVER_STATUS_META[props.server.status])
const showStatus = computed(() => isServerStatusVisible(props.server.status))
</script>

<template>
	<InstanceRowCard
		data-onboarding-id="server-card"
		:name="server.name"
		:version="server.gameVersion"
		:loader="
			server.loaderVersion ? server.serverType + ' ' + server.loaderVersion : server.serverType
		"
		@select="emit('open')"
	>
		<template #prepend>
			<ServerIcon
				:icon-path="server.iconPath"
				:server-type="server.serverType"
				:server-id="server.id"
			/>
		</template>
		<template #append>
			<div class="flex shrink-0 items-center gap-2" @click.stop>
				<TagItem v-if="showStatus">
					<span :class="'font-semibold ' + statusMeta.color">
						{{ formatMessage(statusMeta.label) }}
					</span>
				</TagItem>
				<ButtonStyled v-if="server.status !== 'running'" color="brand" size="small">
					<button type="button" @click="emit('start-stop')">
						<PlayIcon />
						{{ formatMessage(messages.start) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-else color="red" type="outlined" size="small">
					<button type="button" @click="emit('start-stop')">
						<StopCircleIcon />
						{{ formatMessage(messages.stop) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</InstanceRowCard>
</template>
