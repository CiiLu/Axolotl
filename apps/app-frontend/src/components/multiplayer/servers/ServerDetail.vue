<script setup lang="ts">
import {
	ArrowLeftIcon,
	FolderOpenIcon,
	GlobeIcon,
	PlayIcon,
	StopCircleIcon,
	TerminalSquareIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { setEulaAccepted } from '@modrinth/server'
import { ButtonStyled, defineMessages, NavTabs, TagItem, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import { SERVER_STATUS_META } from '@/components/multiplayer/servers/server-status'
import { SERVER_TYPE_META } from '@/components/multiplayer/servers/server-type'
import ServerConsole from '@/components/multiplayer/servers/ServerConsole.vue'
import ServerSettingsPanel from '@/components/multiplayer/servers/ServerSettingsPanel.vue'
import { useMultiplayerSession } from '@/composables/useMultiplayerSession'
import { useServers } from '@/composables/useServers'
import { servers as serversApi } from '@/helpers/servers'
import { openPath } from '@/helpers/utils'

const route = useRoute()
const router = useRouter()
const serverId = route.params.id as string

const { servers, refresh, startServer, stopServer } = useServers()
const multiplayerSession = useMultiplayerSession()
const { formatMessage } = useVIntl()
const eulaModal = ref()
const eulaModalPending = ref('')

const messages = defineMessages({
	console: { id: 'app.servers.detail.console', defaultMessage: 'Console' },
	settings: { id: 'app.servers.detail.settings', defaultMessage: 'Settings' },
	back: { id: 'app.servers.detail.back', defaultMessage: 'Servers' },
	start: { id: 'app.servers.action.start', defaultMessage: 'Start' },
	stop: { id: 'app.servers.action.stop', defaultMessage: 'Stop' },
	openFolder: { id: 'app.servers.action.open-folder', defaultMessage: 'Open folder' },
	share: { id: 'app.servers.action.share', defaultMessage: 'Share online' },
	notFound: {
		id: 'app.servers.detail.not-found',
		defaultMessage: 'This server no longer exists.',
	},
	typeLabel: {
		id: 'app.servers.card.type',
		defaultMessage: '{type} · {version}',
	},
	port: { id: 'app.servers.card.port', defaultMessage: 'Port {port}' },
})

const server = computed(() => servers.value.find((entry) => entry.id === serverId))
const statusMeta = computed(() => (server.value ? SERVER_STATUS_META[server.value.status] : null))

const isLoaded = ref(false)
const hasSeenServer = ref(false)

onMounted(async () => {
	if (servers.value.length === 0) await refresh().catch(() => {})
	isLoaded.value = true
})

// A server disappearing after it was loaded means it was deleted: go back to the list
// instead of showing a "no longer exists" dead end.
watch([server, isLoaded], ([value, loaded]) => {
	if (value) {
		hasSeenServer.value = true
		return
	}
	if (loaded && hasSeenServer.value) void router.replace('/multiplayer/servers')
})

const tabIndex = ref(0)
const tabLinks = computed(() => [
	{ label: formatMessage(messages.console), href: 'console', icon: TerminalSquareIcon },
	{ label: formatMessage(messages.settings), href: 'settings', icon: WrenchIcon },
])

async function toggleRunning() {
	if (!server.value) return
	if (server.value.status === 'running') {
		await stopServer(server.value.id)
	} else {
		await tryStartServer(server.value.id)
	}
}

/** Starts the server; if the EULA is unaccepted, shows the EULA modal first. */
async function tryStartServer(id: string) {
	if (!server.value) return
	if (!server.value.eulaAccepted && server.value.eulaExists) {
		try {
			eulaModalPending.value = await serversApi.readFile(id, 'eula.txt')
			eulaModal.value?.show()
			return
		} catch {
			// No eula.txt: a fresh start will generate it
		}
	}
	await startServer(id)
}

async function onEulaAccept() {
	if (!server.value) return
	try {
		const updated = setEulaAccepted(eulaModalPending.value, true)
		await serversApi.writeFile(server.value.id, 'eula.txt', updated)
		eulaModal.value?.hide()
		await startServer(server.value.id)
	} catch (error) {
		console.error(error)
	}
}

function onEulaDecline() {
	eulaModal.value?.hide()
}

async function shareOnline() {
	if (!server.value?.port) return
	await router.push({ path: '/multiplayer/rooms' })
	void multiplayerSession.hostHongshi(server.value.port, null, null)
}
</script>

<template>
	<div class="multiplayer-fixed-render flex min-h-0 w-full flex-1 flex-col gap-3">
		<div v-if="!server && isLoaded && !hasSeenServer" class="text-secondary">
			{{ formatMessage(messages.notFound) }}
		</div>

		<template v-else>
			<div class="flex min-w-0 shrink-0 flex-wrap items-center justify-between gap-3">
				<div class="flex min-w-0 items-center gap-3">
					<ButtonStyled type="transparent" circular>
						<button
							type="button"
							:aria-label="formatMessage(messages.back)"
							@click="router.push('/multiplayer/servers')"
						>
							<ArrowLeftIcon />
						</button>
					</ButtonStyled>
					<div
						class="flex size-11 shrink-0 items-center justify-center rounded-xl text-base font-bold"
						:style="`--_color: ${SERVER_TYPE_META[server.serverType].colorVar}`"
						:class="[
							'text-[--_color,var(--color-brand)]',
							'bg-[color-mix(in_srgb,var(--_color)_14%,transparent)]',
						]"
					>
						{{ SERVER_TYPE_META[server.serverType].monogram }}
					</div>
					<div class="min-w-0">
						<div class="flex min-w-0 items-center gap-2">
							<h2 class="m-0 truncate text-xl font-semibold text-contrast">
								{{ server.name }}
							</h2>
							<TagItem v-if="statusMeta" class="shrink-0">
								<span :class="`font-semibold ${statusMeta.color}`">
									{{ formatMessage(statusMeta.label) }}
								</span>
							</TagItem>
						</div>
						<div class="mt-0.5 flex min-w-0 items-center gap-2 text-sm text-secondary">
							<span class="truncate">
								{{
									formatMessage(messages.typeLabel, {
										type: server.serverType,
										version: server.gameVersion,
									})
								}}
							</span>
							<span v-if="server.port" class="shrink-0">
								{{ formatMessage(messages.port, { port: server.port }) }}
							</span>
						</div>
					</div>
				</div>

				<div class="flex flex-wrap gap-2">
					<ButtonStyled v-if="server.status !== 'running'" color="brand">
						<button type="button" @click="toggleRunning">
							<PlayIcon />
							{{ formatMessage(messages.start) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-else color="red" type="outlined">
						<button type="button" @click="toggleRunning">
							<StopCircleIcon />
							{{ formatMessage(messages.stop) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="server.status === 'running' && server.port" type="outlined">
						<button type="button" @click="shareOnline">
							<GlobeIcon />
							{{ formatMessage(messages.share) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="outlined">
						<button type="button" @click="openPath(server.path)">
							<FolderOpenIcon />
							{{ formatMessage(messages.openFolder) }}
						</button>
					</ButtonStyled>
				</div>
			</div>

			<NavTabs
				mode="local"
				:active-index="tabIndex"
				:links="tabLinks"
				@tab-click="tabIndex = $event"
			/>

			<div v-if="tabIndex === 0" class="max-h-[calc(100dvh-var(--top-bar-height))] min-h-0 flex-1">
				<ServerConsole :server="server" />
			</div>
			<div v-else class="min-h-0 flex-1 overflow-y-auto pr-1">
				<ServerSettingsPanel :server="server" @deleted="router.push('/multiplayer/servers')" />
			</div>

			<EulaModal
				ref="eulaModal"
				:text="eulaModalPending"
				@accept="onEulaAccept"
				@decline="onEulaDecline"
			/>
		</template>
	</div>
</template>

<style>
/*
 * fixed 渲染模式（服务器详情页）：页面自身不滚动，控制台/设置区内部滚动，
 * 命令输入框始终停留在可视区域内。
 * 显式把 page-transition-grid 定高（100%），让整条 h-full 百分比高度链有确定参照，
 * 避免网格行高被日志内容撑开导致终端无限增高。
 * 只去掉 .app-viewport 的 scrollbar-gutter（避免多余的空滚动条轨道），
 * 保留 overflow: auto 作为兜底——内容万一超出视口仍可滚动，不会被裁切。
 */
.app-viewport:has(.multiplayer-fixed-render) .page-transition-grid {
	height: 100%;
}
.app-viewport:has(.multiplayer-fixed-render) {
	scrollbar-gutter: auto;
}
</style>
