<script setup lang="ts">
import { Button, CopyCode, NavTabs, StyledInput, defineMessages, useVIntl } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref } from 'vue'

const { formatMessage } = useVIntl()
const messages = defineMessages({
	title: { id: 'app.multiplayer.title', defaultMessage: 'Multiplayer' },
	host: { id: 'app.multiplayer.host', defaultMessage: 'Host' },
	join: { id: 'app.multiplayer.join', defaultMessage: 'Join' },
	hostDescription: {
		id: 'app.multiplayer.host-description',
		defaultMessage: 'Create a virtual LAN room so friends can connect directly to your game.',
	},
	lanHint: {
		id: 'app.multiplayer.lan-hint',
		defaultMessage: 'Open your Minecraft world, then press Esc → Open to LAN → choose a port. Terracotta will detect it automatically.',
	},
	joinDescription: {
		id: 'app.multiplayer.join-description',
		defaultMessage: 'Enter a room code to join a friend\'s virtual LAN room.',
	},
	playerName: {
		id: 'app.multiplayer.player-name',
		defaultMessage: 'Player name',
	},
	roomCode: {
		id: 'app.multiplayer.room-code',
		defaultMessage: 'Room code',
	},
	roomCodePlaceholder: {
		id: 'app.multiplayer.room-code-placeholder',
		defaultMessage: 'e.g. U/ABCD-EFGH-IJKL-MNOP',
	},
	startHosting: {
		id: 'app.multiplayer.start-hosting',
		defaultMessage: 'Start hosting',
	},
	joinRoom: {
		id: 'app.multiplayer.join-room',
		defaultMessage: 'Join room',
	},
	stop: {
		id: 'app.multiplayer.stop',
		defaultMessage: 'Stop',
	},
	disconnect: {
		id: 'app.multiplayer.disconnect',
		defaultMessage: 'Disconnect',
	},
	copyRoomCode: {
		id: 'app.multiplayer.copy-room-code',
		defaultMessage: 'Copy room code',
	},
	back: {
		id: 'app.multiplayer.back',
		defaultMessage: 'Back',
	},
	statusIdle: {
		id: 'app.multiplayer.status.idle',
		defaultMessage: 'Not connected',
	},
	statusStarting: {
		id: 'app.multiplayer.status.starting',
		defaultMessage: 'Starting...',
	},
	statusWaiting: {
		id: 'app.multiplayer.status.waiting',
		defaultMessage: 'Waiting...',
	},
	statusHostScanning: {
		id: 'app.multiplayer.status.host-scanning',
		defaultMessage: 'Creating room...',
	},
	statusHostStarting: {
		id: 'app.multiplayer.status.host-starting',
		defaultMessage: 'Starting host...',
	},
	statusHostReady: {
		id: 'app.multiplayer.status.host-ready',
		defaultMessage: 'Room ready',
	},
	statusGuestConnecting: {
		id: 'app.multiplayer.status.guest-connecting',
		defaultMessage: 'Joining room...',
	},
	statusGuestStarting: {
		id: 'app.multiplayer.status.guest-starting',
		defaultMessage: 'Connecting as guest...',
	},
	statusGuestReady: {
		id: 'app.multiplayer.status.guest-ready',
		defaultMessage: 'Connected to room',
	},
	statusError: {
		id: 'app.multiplayer.status.error',
		defaultMessage: 'Error',
	},
	statusFatal: {
		id: 'app.multiplayer.status.fatal',
		defaultMessage: 'Fatal error',
	},
	statusDownloading: {
		id: 'app.multiplayer.status.downloading',
		defaultMessage: 'Downloading...',
	},
	players: {
		id: 'app.multiplayer.players',
		defaultMessage: 'Players',
	},
	playersInRoom: {
		id: 'app.multiplayer.players-in-room',
		defaultMessage: '{count} player(s) in room',
	},
	notRunning: {
		id: 'app.multiplayer.not-running',
		defaultMessage: 'Multiplayer service is not running. Start hosting or join a room to begin.',
	},
	notRunningTitle: {
		id: 'app.multiplayer.not-running-title',
		defaultMessage: 'Start a multiplayer session',
	},
	shareCode: {
		id: 'app.multiplayer.share-code',
		defaultMessage: 'Share this code with friends to let them join:',
	},
	hostLabel: {
		id: 'app.multiplayer.host-label',
		defaultMessage: 'Host',
	},
	guestLabel: {
		id: 'app.multiplayer.guest-label',
		defaultMessage: 'Guest',
	},
	platformInfo: {
		id: 'app.multiplayer.platform-info',
		defaultMessage: 'Current platform: {platform}',
	},
	binaryNotFound: {
		id: 'app.multiplayer.binary-not-found',
		defaultMessage: 'Terracotta binary not found. Please download it and place it at:',
	},
	downloadTerracotta: {
		id: 'app.multiplayer.download-terracotta',
		defaultMessage: 'Download Terracotta',
	},
	retry: {
		id: 'app.multiplayer.retry',
		defaultMessage: 'Retry',
	},
	checkNetwork: {
		id: 'app.multiplayer.check-network',
		defaultMessage: 'Check your network connection',
	},
	downloadProgress: {
		id: 'app.multiplayer.download-progress',
		defaultMessage: 'Download progress',
	},
	verifying: {
		id: 'app.multiplayer.verifying',
		defaultMessage: 'Verifying...',
	},
	connecting: {
		id: 'app.multiplayer.connecting',
		defaultMessage: 'Connecting...',
	},
	errorNetwork: {
		id: 'app.multiplayer.error.network',
		defaultMessage: 'Network error',
	},
	errorInstall: {
		id: 'app.multiplayer.error.install',
		defaultMessage: 'Installation error',
	},
	errorTerracotta: {
		id: 'app.multiplayer.error.terracotta',
		defaultMessage: 'Terracotta error',
	},
	errorUnknown: {
		id: 'app.multiplayer.error.unknown',
		defaultMessage: 'Unknown error',
	},
	errorOs: {
		id: 'app.multiplayer.error.os',
		defaultMessage: 'System error',
	},
	poweredByTerracotta: {
		id: 'app.multiplayer.powered-by-terracotta',
		defaultMessage: 'Powered by Terracotta | 陶瓦联机',
	},
	startTerracotta: {
		id: 'app.multiplayer.start-terracotta',
		defaultMessage: 'Start Terracotta',
	},
	startDescription: {
		id: 'app.multiplayer.start-description',
		defaultMessage: 'Start the multiplayer service to host games or join friends\' rooms.',
	},
	loading: {
		id: 'app.multiplayer.loading',
		defaultMessage: 'Initializing...',
	},
	noPlayers: {
		id: 'app.multiplayer.no-players',
		defaultMessage: 'No players in room',
	},
})

interface PlayerInfo {
	machine_id: string
	name: string
	vendor: string
	kind: string
}

interface TerracottaState {
	status: string
	http_port: number | null
	room_code: string | null
	server_port: number | null
	players: PlayerInfo[]
	download_progress: number | null
	download_stage: string | null
	binary_installed: boolean
	error_type: string | null
	error_message: string | null
	profile_index: number | null
}

const tabIndex = ref(0)
const playerName = ref('')
const roomCodeInput = ref('')
const state = ref<TerracottaState | null>(null)
const binaryInstalled = ref(false)
const platformKey = ref('')
const isMounted = ref(false)

let pollInterval: ReturnType<typeof setInterval> | null = null

const tabLinks = computed(() => [
	{ label: formatMessage(messages.host), href: 'host' },
	{ label: formatMessage(messages.join), href: 'join' },
])

const isRunning = computed(() => !!state.value?.http_port)

const statusText = computed(() => {
	if (!state.value) return ''
	const statusMap: Record<string, any> = {
		idle: messages.statusIdle,
		starting: messages.statusStarting,
		waiting: messages.statusWaiting,
		host_scanning: messages.statusHostScanning,
		host_starting: messages.statusHostStarting,
		host_ready: messages.statusHostReady,
		guest_connecting: messages.statusGuestConnecting,
		guest_starting: messages.statusGuestStarting,
		guest_ready: messages.statusGuestReady,
		error: messages.statusError,
		fatal: messages.statusFatal,
		downloading: messages.statusDownloading,
	}
	return formatMessage(statusMap[state.value.status] ?? messages.statusIdle)
})

const statusIndicatorClass = computed(() => {
	const s = state.value?.status
	if (s === 'host_ready' || s === 'guest_ready') return 'bg-green-500'
	if (s === 'error' || s === 'fatal') return 'bg-red-500'
	if (s === 'downloading') return 'bg-yellow-500 animate-pulse'
	if (s === 'starting' || s === 'host_scanning' || s === 'host_starting' || s === 'guest_connecting' || s === 'guest_starting') return 'bg-yellow-500 animate-pulse'
	return 'bg-yellow-500 animate-pulse'
})

const playerCount = computed(() => state.value?.players?.length ?? 0)

const binaryPathHint = computed(() => {
	const name = platformKey.value?.includes('windows') ? 'terracotta.exe' : 'terracotta'
	return `<launcher_dir>/terracotta/${name}`
})

const downloadStageText = computed(() => {
	if (state.value?.download_stage) {
		if (state.value.download_stage === 'verifying') return formatMessage(messages.verifying)
		if (state.value.download_stage === 'complete') return ''
		if (state.value.download_stage === 'preparing') return formatMessage(messages.connecting)
		return state.value.download_stage
	}
	if (state.value?.status === 'downloading') {
		if (state.value.download_progress === null || state.value.download_progress === 0) return formatMessage(messages.connecting)
		if (state.value.download_progress! < 100) return formatMessage(messages.downloadProgress)
		return formatMessage(messages.verifying)
	}
	return ''
})

const errorTypeLabel = computed(() => {
	const et = state.value?.error_type
	switch (et) {
		case 'network': return formatMessage(messages.errorNetwork)
		case 'install': return formatMessage(messages.errorInstall)
		case 'terracotta': return formatMessage(messages.errorTerracotta)
		case 'os': return formatMessage(messages.errorOs)
		default: return formatMessage(messages.errorUnknown)
	}
})

const isRecoverable = computed(() => {
	const et = state.value?.error_type
	if (!et) return state.value?.status === 'error'
	return et !== 'fatal' && et !== 'os'
})

async function pollState() {
	try {
		const result = await invoke<any>('plugin:terracotta|terracotta_get_state')
		if (!isMounted.value) return
		state.value = result as TerracottaState
		binaryInstalled.value = result.binary_installed ?? false
	} catch (e: any) {
		if (!isMounted.value) return
		console.error(e)
	}
}

function startPolling(intervalMs = 1000) {
	if (pollInterval) clearInterval(pollInterval)
	pollInterval = setInterval(() => {
		if (isMounted.value) pollState()
	}, intervalMs)
}

function stopPolling() {
	if (pollInterval) {
		clearInterval(pollInterval)
		pollInterval = null
	}
}

async function startTerracotta() {
	try {
		await invoke('plugin:terracotta|terracotta_start', { autoDownload: true })
		startPolling()
	} catch (e: any) {
		console.error(e)
	}
}

async function stopTerracotta() {
	try {
		await invoke('plugin:terracotta|terracotta_stop')
		stopPolling()
		await pollState()
	} catch (e: any) {
		console.error(e)
		stopPolling()
		await pollState()
	}
}

async function hostGame() {
	if (!playerName.value.trim()) {
		console.warn('Please enter a player name')
		return
	}
	try {
		await invoke('plugin:terracotta|terracotta_host', {
			playerName: playerName.value.trim(),
		})
	} catch (e: any) {
		console.error(e)
	}
}

async function joinGame() {
	if (!playerName.value.trim()) {
		console.warn('Please enter a player name')
		return
	}
	if (!roomCodeInput.value.trim()) {
		console.warn('Please enter a room code')
		return
	}
	try {
		const parsed = await invoke<string>('plugin:terracotta|terracotta_parse_room_code', {
			roomCode: roomCodeInput.value.trim(),
		})
		await invoke('plugin:terracotta|terracotta_join', {
			playerName: playerName.value.trim(),
			roomCode: parsed,
		})
	} catch (e: any) {
		console.error(e)
	}
}

async function resetState() {
	try {
		await invoke('plugin:terracotta|terracotta_reset')
		await pollState()
	} catch (e: any) {
		console.error(e)
	}
}

async function downloadTerracotta() {
	try {
		startPolling(500)
		await invoke('plugin:terracotta|terracotta_download')
	} catch (e: any) {
		stopPolling()
		if (isMounted.value) {
			console.error(e)
		}
	}
}

onMounted(async () => {
	isMounted.value = true
	await pollState()
	try {
		platformKey.value = await invoke<string>('plugin:terracotta|terracotta_get_platform_key')
	} catch {
		platformKey.value = 'unknown'
	}
	try {
		const name = await invoke<string>('plugin:terracotta|terracotta_get_player_name')
		if (name && isMounted.value) playerName.value = name
	} catch {
	}
})

onUnmounted(() => {
	isMounted.value = false
	stopPolling()
})
</script>

<template>
	<div class="p-6 flex flex-col gap-6 max-w-2xl mx-auto w-full">
		<h1 class="text-2xl font-bold">
			{{ formatMessage(messages.title) }}
		</h1>

		<template v-if="!state">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5 text-center">
				<div class="flex items-center justify-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-yellow-500 animate-pulse" />
					<div class="text-lg font-semibold">
						{{ binaryInstalled ? formatMessage(messages.notRunningTitle) : formatMessage(messages.loading) }}
					</div>
				</div>
				<template v-if="binaryInstalled">
					<div class="text-sm text-secondary mb-4">
						{{ formatMessage(messages.notRunning) }}
					</div>
					<Button @click="startTerracotta">
						{{ formatMessage(messages.startTerracotta) }}
					</Button>
				</template>
			</div>
		</template>

		<template v-else-if="!state.binary_installed">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5 text-center">
				<div class="text-lg font-semibold mb-2">
					{{ formatMessage(messages.notRunningTitle) }}
				</div>
				<div class="text-sm text-secondary">
					{{ formatMessage(messages.notRunning) }}
				</div>
				<div class="mt-4 text-xs text-secondary space-y-1">
					<div>{{ formatMessage(messages.platformInfo, { platform: platformKey }) }}</div>
					<div>{{ formatMessage(messages.binaryNotFound) }}</div>
					<code class="text-xs bg-surface-5 px-2 py-1 rounded inline-block mt-1 select-all">
						{{ binaryPathHint }}
					</code>
				</div>

				<div v-if="state.status === 'downloading'" class="mt-3">
					<div class="flex items-center gap-2 mb-2 justify-center">
						<div class="w-3 h-3 rounded-full bg-yellow-500 animate-pulse flex-shrink-0" />
						<span class="text-sm text-secondary">{{ downloadStageText || statusText }}</span>
					</div>
					<div class="h-2 bg-surface-5 rounded-full overflow-hidden max-w-xs mx-auto">
						<div
							class="h-full bg-brand rounded-full transition-all duration-300"
							:style="{ width: (state.download_progress || 0) + '%' }"
						/>
					</div>
					<div class="text-xs text-secondary mt-1">{{ state.download_progress || 0 }}%</div>
				</div>

				<Button
					v-if="state.status !== 'downloading'"
					class="mt-3"
					@click="downloadTerracotta"
				>
					{{ formatMessage(messages.downloadTerracotta) }}
				</Button>
			</div>
		</template>

		<template v-else-if="state.status === 'starting'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3">
					<div class="w-4 h-4 rounded-full bg-yellow-500 animate-pulse" />
					<div class="font-semibold text-lg">
						{{ statusText }}
					</div>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'downloading'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-yellow-500 animate-pulse" />
					<div class="font-semibold text-lg">
						{{ statusText }}
					</div>
				</div>
				<div v-if="state.download_progress !== null" class="mb-2">
					<div class="flex items-center justify-between text-sm text-secondary mb-1">
						<span>{{ downloadStageText }}</span>
						<span>{{ state.download_progress }}%</span>
					</div>
					<div class="h-2 bg-surface-5 rounded-full overflow-hidden">
						<div
							class="h-full bg-brand rounded-full transition-all duration-300"
							:style="{ width: (state.download_progress || 0) + '%' }"
						/>
					</div>
				</div>
			</div>
		</template>

		<template v-else-if="isRunning && (state.status === 'idle' || state.status === 'waiting')">
			<div class="bg-bg-raised rounded-xl border border-surface-5 overflow-hidden">
				<NavTabs
					mode="local"
					:active-index="tabIndex"
					:links="tabLinks"
					@tab-click="tabIndex = $event"
				/>
				<div class="p-6 flex flex-col gap-4">
					<StyledInput
						v-model="playerName"
						:placeholder="formatMessage(messages.playerName)"
					/>

					<StyledInput
						v-if="tabIndex === 1"
						v-model="roomCodeInput"
						:placeholder="formatMessage(messages.roomCodePlaceholder)"
					/>

					<div class="text-sm text-secondary">
						{{
							tabIndex === 0
								? formatMessage(messages.hostDescription)
								: formatMessage(messages.joinDescription)
						}}
					</div>

					<div class="flex gap-2">
						<Button
							v-if="tabIndex === 0"
							@click="hostGame"
						>
							{{ formatMessage(messages.startHosting) }}
						</Button>
						<Button
							v-else
							@click="joinGame"
						>
							{{ formatMessage(messages.joinRoom) }}
						</Button>
						<Button color="danger" @click="stopTerracotta">
							{{ formatMessage(messages.stop) }}
						</Button>
					</div>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'host_scanning' || state.status === 'host_starting'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-yellow-500 animate-pulse" />
					<div class="font-semibold text-lg">
						{{ statusText }}
					</div>
				</div>
				<div class="text-sm text-secondary mb-4 bg-surface-5 rounded-lg p-3">
					{{ formatMessage(messages.lanHint) }}
				</div>
				<div class="flex gap-2">
					<Button @click="resetState">
						{{ formatMessage(messages.back) }}
					</Button>
					<Button color="danger" @click="stopTerracotta">
						{{ formatMessage(messages.stop) }}
					</Button>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'host_ready'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-green-500" />
					<div>
						<div class="font-semibold text-lg">
							{{ statusText }}
						</div>
						<div v-if="state.room_code" class="text-sm text-secondary mt-1">
							{{ formatMessage(messages.shareCode) }}
						</div>
					</div>
				</div>

				<div v-if="state.room_code" class="mb-4">
					<div class="flex items-center gap-2 bg-surface-5 rounded-lg p-3">
						<code class="text-lg font-mono select-all flex-1">{{ state.room_code }}</code>
						<CopyCode :text="state.room_code" />
					</div>
				</div>

				<div class="mb-4">
					<div class="text-sm font-semibold text-secondary mb-2">
						{{ formatMessage(messages.players) }}
					</div>
					<div v-if="state.players.length > 0" class="flex flex-col gap-1">
						<div
							v-for="(player, idx) in state.players"
							:key="idx"
							class="flex items-center gap-2 text-sm bg-surface-5 rounded-lg px-3 py-1.5"
						>
							<div class="w-2 h-2 rounded-full bg-green-400 flex-shrink-0" />
							<span>{{ player.name }}</span>
							<span
								class="text-secondary text-xs px-1.5 py-0.5 bg-surface-10 rounded ml-auto"
							>
								{{ player.kind === 'HOST' ? formatMessage(messages.hostLabel) : formatMessage(messages.guestLabel) }}
							</span>
						</div>
					</div>
					<div v-else class="text-sm text-secondary">
						{{ formatMessage(messages.noPlayers) }}
					</div>
				</div>

				<div class="flex gap-2">
					<Button @click="resetState">
						{{ formatMessage(messages.back) }}
					</Button>
					<Button color="danger" @click="stopTerracotta">
						{{ formatMessage(messages.stop) }}
					</Button>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'guest_connecting' || state.status === 'guest_starting'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-yellow-500 animate-pulse" />
					<div class="font-semibold text-lg">
						{{ statusText }}
					</div>
				</div>
				<div class="flex gap-2">
					<Button @click="resetState">
						{{ formatMessage(messages.back) }}
					</Button>
					<Button color="danger" @click="stopTerracotta">
						{{ formatMessage(messages.stop) }}
					</Button>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'guest_ready'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-green-500" />
					<div>
						<div class="font-semibold text-lg">
							{{ statusText }}
						</div>
					</div>
				</div>

				<div class="mb-4">
					<div class="text-sm font-semibold text-secondary mb-2">
						{{ formatMessage(messages.players) }}
					</div>
					<div v-if="state.players.length > 0" class="flex flex-col gap-1">
						<div
							v-for="(player, idx) in state.players"
							:key="idx"
							class="flex items-center gap-2 text-sm bg-surface-5 rounded-lg px-3 py-1.5"
						>
							<div class="w-2 h-2 rounded-full bg-green-400 flex-shrink-0" />
							<span>{{ player.name }}</span>
							<span
								class="text-secondary text-xs px-1.5 py-0.5 bg-surface-10 rounded ml-auto"
							>
								{{ player.kind === 'HOST' ? formatMessage(messages.hostLabel) : formatMessage(messages.guestLabel) }}
							</span>
						</div>
					</div>
					<div v-else class="text-sm text-secondary">
						{{ formatMessage(messages.noPlayers) }}
					</div>
				</div>

				<div class="flex gap-2">
					<Button @click="resetState">
						{{ formatMessage(messages.back) }}
					</Button>
					<Button color="danger" @click="stopTerracotta">
						{{ formatMessage(messages.stop) }}
					</Button>
				</div>
			</div>
		</template>

		<template v-else-if="state.status === 'error' || state.status === 'fatal'">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-4 h-4 rounded-full bg-red-500" />
					<div class="font-semibold text-lg">
						{{ statusText }}
					</div>
				</div>
				<div class="bg-red-900/20 border border-red-500/30 rounded-lg p-4">
					<div class="text-sm text-red-400 font-semibold mb-1">
						{{ errorTypeLabel }}
					</div>
					<div v-if="state.error_message" class="text-sm text-red-300">
						{{ state.error_message }}
					</div>
					<div class="flex gap-2 mt-3">
						<Button v-if="isRecoverable" size="small" @click="resetState">
							{{ formatMessage(messages.retry) }}
						</Button>
						<Button size="small" color="danger" @click="stopTerracotta">
							{{ formatMessage(messages.stop) }}
						</Button>
					</div>
				</div>
			</div>
		</template>

		<template v-else-if="!isRunning">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5 text-center">
				<div class="text-lg font-semibold mb-4">
					{{ formatMessage(messages.notRunningTitle) }}
				</div>
				<div class="text-sm text-secondary mb-4">
					{{ formatMessage(messages.startDescription) }}
				</div>
				<Button @click="startTerracotta">
					{{ formatMessage(messages.startTerracotta) }}
				</Button>
			</div>
		</template>

		<div class="text-center mt-6">
			<span class="text-xs text-tertiary">{{ formatMessage(messages.poweredByTerracotta) }}</span>
		</div>
	</div>
</template>
