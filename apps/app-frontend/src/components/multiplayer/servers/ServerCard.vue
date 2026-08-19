<script setup lang="ts">
import { BoxIcon, PlayIcon, StopCircleIcon } from '@modrinth/assets'
import { ButtonStyled, Card, defineMessages, TagItem, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { SERVER_STATUS_META } from '@/components/multiplayer/servers/server-status'
import { SERVER_TYPE_META } from '@/components/multiplayer/servers/server-type'
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
	typeLabel: {
		id: 'app.servers.card.type',
		defaultMessage: '{type} · {version}',
	},
	port: { id: 'app.servers.card.port', defaultMessage: 'Port {port}' },
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
})

const statusMeta = computed(() => SERVER_STATUS_META[props.server.status])
</script>

<template>
	<Card
		data-onboarding-id="server-card"
		class="!m-0 cursor-pointer transition-colors hover:border-surface-5"
		@click="emit('open')"
	>
		<div class="flex flex-col gap-3">
			<div class="flex min-w-0 items-start justify-between gap-3">
				<div class="flex min-w-0 items-center gap-3">
					<div
						class="flex size-10 shrink-0 items-center justify-center rounded-xl text-sm font-bold"
						:style="`--_color: ${SERVER_TYPE_META[server.serverType].colorVar}`"
						:class="[
							'text-[--_color,var(--color-brand)]',
							'bg-[color-mix(in_srgb,var(--_color)_14%,transparent)]',
						]"
					>
						{{ SERVER_TYPE_META[server.serverType].monogram }}
					</div>
					<div class="min-w-0">
						<span class="block truncate font-semibold text-contrast">
							{{ server.name }}
						</span>
						<span class="block truncate text-sm text-secondary">
							{{
								formatMessage(messages.typeLabel, {
									type: server.serverType,
									version: server.gameVersion,
								})
							}}
						</span>
					</div>
				</div>
				<TagItem>
					<span :class="`font-semibold ${statusMeta.color}`">
						{{ formatMessage(statusMeta.label) }}
					</span>
				</TagItem>
			</div>

			<div class="flex items-center justify-between gap-3">
				<span v-if="server.port" class="flex items-center gap-1 text-sm text-secondary">
					<BoxIcon class="size-4" />
					{{ formatMessage(messages.port, { port: server.port }) }}
				</span>
				<span v-else />

				<div class="flex gap-2" @click.stop>
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
			</div>
		</div>
	</Card>
</template>
