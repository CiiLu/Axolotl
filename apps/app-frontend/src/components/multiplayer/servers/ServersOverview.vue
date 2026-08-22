<script setup lang="ts">
import { PlusIcon, RefreshCwIcon, ServerIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, EmptyState, useVIntl } from '@modrinth/ui'
import { onMounted, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'
import { useRouter } from 'vue-router'

import CreateServerModal from '@/components/multiplayer/servers/CreateServerModal.vue'
import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import ServerCard from '@/components/multiplayer/servers/ServerCard.vue'
import { type ServerView, useServers } from '@/composables/useServers'
import { useServerLifecycle } from '@/composables/useServerLifecycle'

const router = useRouter()
const { formatMessage } = useVIntl()
const { servers, isRefreshing, refresh, stopServer } = useServers()
const { eulaModal, eulaText, tryStartServer, acceptEula, declineEula } = useServerLifecycle()
const createModal = useTemplateRef<ComponentExposed<typeof CreateServerModal>>('createModal')

const messages = defineMessages({
	create: { id: 'app.servers.create.title', defaultMessage: 'Create server' },
	refresh: { id: 'app.servers.refresh', defaultMessage: 'Refresh' },
	emptyHeading: {
		id: 'app.servers.empty.heading',
		defaultMessage: 'No servers yet',
	},
	emptyDescription: {
		id: 'app.servers.empty.description',
		defaultMessage: 'Create a server to play with friends, right from the launcher.',
	},
	count: {
		id: 'app.servers.count',
		defaultMessage: '{count, plural, =0 {No servers yet} one {# server} other {# servers}}',
	},
	loading: { id: 'app.servers.loading', defaultMessage: 'Loading servers...' },
})

onMounted(() => {
	void refresh()
})

function openServer(id: string) {
	void router.push('/multiplayer/servers/' + encodeURIComponent(id))
}

async function toggleRunning(server: ServerView) {
	if (server.status === 'running') {
		await stopServer(server.id)
	} else {
		await tryStartServer(server)
	}
}
</script>

<template>
	<div data-onboarding-id="servers-overview" class="flex min-h-0 w-full flex-1 flex-col gap-4">
		<div class="flex items-center justify-between gap-3">
			<span class="flex items-center gap-2 text-sm text-secondary">
				<SpinnerIcon v-if="isRefreshing" class="size-4 animate-spin" />
				<ServerIcon v-else class="size-4" />
				{{
					isRefreshing
						? formatMessage(messages.loading)
						: formatMessage(messages.count, { count: servers.length })
				}}
			</span>
			<div class="flex gap-2">
				<ButtonStyled type="outlined">
					<button type="button" :disabled="isRefreshing" @click="refresh()">
						<RefreshCwIcon :class="{ 'animate-spin': isRefreshing }" />
						{{ formatMessage(messages.refresh) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						type="button"
						data-onboarding-id="create-server-button"
						@click="createModal?.show()"
					>
						<PlusIcon />
						{{ formatMessage(messages.create) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<EmptyState
			v-if="servers.length === 0 && !isRefreshing"
			type="empty"
			:heading="formatMessage(messages.emptyHeading)"
			:description="formatMessage(messages.emptyDescription)"
		>
			<ButtonStyled color="brand" size="large">
				<button type="button" @click="createModal?.show()">
					<ServerIcon />
					{{ formatMessage(messages.create) }}
				</button>
			</ButtonStyled>
		</EmptyState>

		<div v-else class="flex max-w-3xl flex-col gap-2">
			<ServerCard
				v-for="entry in servers"
				:key="entry.id"
				:server="entry"
				@open="openServer(entry.id)"
				@start-stop="toggleRunning(entry)"
			/>
		</div>

		<CreateServerModal ref="createModal" />
		<EulaModal ref="eulaModal" :text="eulaText" @accept="acceptEula" @decline="declineEula" />
	</div>
</template>
