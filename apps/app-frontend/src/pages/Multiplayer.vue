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

const tab = ref<'host' | 'join'>('host')
const playerName = ref('')
const roomCodeInput = ref('')
const localError = ref('')
const state = ref<TerracottaState | null>(null)
const platformKey = ref('')
const isMounted = ref(false)
const showStopConfirm = ref(false)

let pollInterval: ReturnType<typeof setInterval> | null = null

const statusText = computed(() => {
	if (!state.value) return formatMessage(messages.notRunning)
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
	return 'bg-yellow-500 animate-pulse'
})

const isConnected = computed(() => {
	const s = state.value?.status
	return s === 'host_ready' || s === 'guest_ready'
})

const isRunning = computed(() => {
	return !!state.value?.http_port
})

const isHosting = computed(() => {
	const s = state.value?.status
	return s === 'host_scanning' || s === 'host_starting' || s === 'host_ready'
})

const playerCount = computed(() => {
	return state.value?.players?.length ?? 0
})

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

const currentError = computed(() => {
	return state.value?.error_message || localError.value || ''
})

async function pollState() {
	try {
		const result = await invoke<any>('plugin:terracotta|terracotta_get_state')
		if (!isMounted.value) return
		state.value = result as TerracottaState
		if (localError.value) localError.value = ''
	} catch (e: any) {
		if (!isMounted.value) return
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Unknown error'
	}
}

async function startTerracotta() {
	localError.value = ''
	try {
		await invoke('plugin:terracotta|terracotta_start', { autoDownload: true })
		if (!pollInterval) {
			pollInterval = setInterval(() => {
				if (isMounted.value) pollState()
			}, 1000)
		}
	} catch (e: any) {
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Failed to start Terracotta'
	}
}

async function stopTerracotta() {
	showStopConfirm.value = false
	try {
		await invoke('plugin:terracotta|terracotta_stop')
		if (pollInterval) {
			clearInterval(pollInterval)
			pollInterval = null
		}
		state.value = null
		localError.value = ''
	} catch (e: any) {
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Failed to stop Terracotta'
	}
}

async function hostGame() {
	localError.value = ''
	if (!playerName.value.trim()) {
		localError.value = 'Please enter a player name'
		return
	}
	if (!state.value?.http_port) {
		await startTerracotta()
		if (localError.value) return
	}
	try {
		await invoke('plugin:terracotta|terracotta_host', {
			playerName: playerName.value.trim(),
		})
	} catch (e: any) {
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Failed to host game'
	}
}

async function joinGame() {
	localError.value = ''
	if (!playerName.value.trim()) {
		localError.value = 'Please enter a player name'
		return
	}
	if (!roomCodeInput.value.trim()) {
		localError.value = 'Please enter a room code'
		return
	}
	if (!state.value?.http_port) {
		await startTerracotta()
		if (localError.value) return
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
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Failed to join game'
	}
}

async function resetState() {
	try {
		await invoke('plugin:terracotta|terracotta_reset')
		await pollState()
	} catch (e: any) {
		localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Failed to reset state'
	}
}

async function downloadTerracotta() {
	localError.value = ''
	try {
		if (!pollInterval) {
			pollInterval = setInterval(() => {
				if (isMounted.value) pollState()
			}, 500)
		}
		await invoke('plugin:terracotta|terracotta_download')
		if (isMounted.value) await startTerracotta()
	} catch (e: any) {
		if (pollInterval) {
			clearInterval(pollInterval)
			pollInterval = null
		}
		if (isMounted.value) {
			localError.value = typeof e === 'string' ? e : e?.message || e?.toString() || 'Download failed'
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
	if (state.value?.binary_installed && !state.value?.http_port) {
		await startTerracotta()
	}
})

onUnmounted(() => {
	isMounted.value = false
	if (pollInterval) {
		clearInterval(pollInterval)
		pollInterval = null
	}
})
</script>

<template>
	<div class="p-6 flex flex-col gap-6 max-w-2xl mx-auto w-full">
		<h1 class="text-2xl font-bold">
			{{ formatMessage(messages.title) }}
		</h1>

		<template v-if="isConnected">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-3 h-3 rounded-full flex-shrink-0" :class="statusIndicatorClass" />
					<div>
						<div class="font-semibold text-lg">{{ statusText }}</div>
						<div v-if="isHosting && state?.room_code" class="text-sm text-secondary mt-1">
							{{ formatMessage(messages.shareCode) }}
						</div>
					</div>
				</div>

				<div v-if="isHosting && state?.room_code" class="mb-4">
					<div class="flex items-center gap-2 bg-surface-5 rounded-lg p-3">
						<code class="text-lg font-mono select-all flex-1">{{ state.room_code }}</code>
						<CopyCode :text="state.room_code" />
					</div>
				</div>

				<div class="mb-4">
					<div class="text-sm font-semibold text-secondary mb-2">
						{{ formatMessage(messages.playersInRoom, { count: playerCount }) }}
					</div>
					<div v-if="playerCount > 0" class="flex flex-col gap-1">
						<div
							v-for="(player, idx) in state?.players"
							:key="idx"
							class="flex items-center gap-2 text-sm bg-surface-5 rounded-lg px-3 py-1.5"
						>
							<div class="w-2 h-2 rounded-full bg-green-400 flex-shrink-0" />
							<span>{{ player.name }}</span>
							<span class="text-secondary text-xs px-1.5 py-0.5 bg-surface-10 rounded ml-auto">
								{{ player.kind === 'HOST' ? formatMessage(messages.hostLabel) : formatMessage(messages.guestLabel) }}
							</span>
						</div>
					</div>
					<div v-else class="text-sm text-secondary">
						{{ formatMessage(messages.statusGuestConnecting) }}
					</div>
				</div>

				<div class="flex gap-2">
					<template v-if="!showStopConfirm">
						<Button color="danger" @click="showStopConfirm = true">
							{{ formatMessage(messages.stop) }}
						</Button>
					</template>
					<template v-else>
						<Button color="danger" @click="stopTerracotta">
							{{ formatMessage(messages.disconnect) }}
						</Button>
						<Button @click="showStopConfirm = false">
							Cancel
						</Button>
					</template>
				</div>
			</div>
		</template>

		<template v-else-if="isRunning">
			<div class="bg-bg-raised rounded-xl p-6 border border-surface-5">
				<div class="flex items-center gap-3 mb-4">
					<div class="w-3 h-3 rounded-full flex-shrink-0" :class="statusIndicatorClass" />
					<div>
						<div class="font-semibold">{{ statusText }}</div>
						<div v-if="downloadStageText" class="text-sm text-secondary mt-0.5">
							{{ downloadStageText }}
						</div>
					</div>
				</div>

				<div
					v-if="state?.status === 'downloading' && state?.download_progress !== null"
					class="mb-4"
				>
					<div class="h-2 bg-surface-5 rounded-full overflow-hidden">
						<div
							class="h-full bg-brand rounded-full transition-all duration-300"
							:style="{ width: (state.download_progress || 0) + '%' }"
						/>
					</div>
					<div class="text-xs text-secondary mt-1">{{ state.download_progress }}%</div>
				</div>

				<div
					v-if="state?.status === 'error' || state?.status === 'fatal'"
					class="mb-4"
				>
					<div class="bg-red-900/20 border border-red-500/30 rounded-lg p-4">
						<div class="text-sm text-red-400 font-semibold mb-1">
							{{ errorTypeLabel }}
						</div>
						<div v-if="currentError" class="text-sm text-red-300">
							{{ currentError }}
						</div>
						<div v-if="state?.error_type !== 'network' && state?.status === 'error'" class="mt-2">
							<a class="text-xs text-red-400 underline cursor-pointer" @click="localError = formatMessage(messages.checkNetwork)">
								{{ formatMessage(messages.checkNetwork) }}
							</a>
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

				<div
					v-if="localError && state?.status !== 'error' && state?.status !== 'fatal'"
					class="text-red-500 text-sm mb-4"
				>
					{{ localError }}
				</div>

				<div
					v-if="state?.status !== 'error' && state?.status !== 'fatal' && state?.status !== 'downloading'"
					class="flex gap-2"
				>
					<Button @click="stopTerracotta">
						{{ formatMessage(messages.stop) }}
					</Button>
				</div>
			</div>
		</template>

		<template v-else>
			<div
				v-if="!state?.binary_installed"
				class="bg-bg-raised rounded-xl p-6 border border-surface-5 text-center"
			>
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

				<div v-if="state?.status === 'downloading'" class="mt-3">
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
					v-if="state?.status !== 'downloading'"
					class="mt-3"
					size="small"
					@click="downloadTerracotta"
				>
					{{ formatMessage(messages.downloadTerracotta) }}
				</Button>

				<div v-if="localError && state?.status !== 'downloading'" class="text-red-500 text-sm mt-3">
					{{ localError }}
				</div>
			</div>

			<div v-else class="bg-bg-raised rounded-xl border border-surface-5 overflow-hidden">
				<NavTabs
					v-model="tab"
					:tabs="[
						{ id: 'host', label: formatMessage(messages.host) },
						{ id: 'join', label: formatMessage(messages.join) },
					]"
				/>
				<div class="p-6 flex flex-col gap-4">
					<StyledInput
						v-model="playerName"
						:placeholder="formatMessage(messages.playerName)"
					/>

					<StyledInput
						v-if="tab === 'join'"
						v-model="roomCodeInput"
						:placeholder="formatMessage(messages.roomCodePlaceholder)"
					/>

					<div class="text-sm text-secondary">
						{{ tab === 'host' ? formatMessage(messages.hostDescription) : formatMessage(messages.joinDescription) }}
					</div>

					<div v-if="localError" class="text-red-500 text-sm">{{ localError }}</div>

					<Button
						v-if="tab === 'host'"
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
				</div>
			</div>
		</template>
		<div class="text-center mt-6">
			<span class="text-xs text-tertiary">{{ formatMessage(messages.poweredByTerracotta) }}</span>
		</div>
	</div>
</template>
