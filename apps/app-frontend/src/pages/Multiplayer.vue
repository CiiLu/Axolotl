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
	statusHostScanning: {
		id: 'app.multiplayer.status.host-scanning',
		defaultMessage: 'Creating room...',
	},
	statusHostReady: {
		id: 'app.multiplayer.status.host-ready',
		defaultMessage: 'Room ready',
	},
	statusGuestConnecting: {
		id: 'app.multiplayer.status.guest-connecting',
		defaultMessage: 'Joining room...',
	},
	statusGuestReady: {
		id: 'app.multiplayer.status.guest-ready',
		defaultMessage: 'Connected to room',
	},
	statusError: {
		id: 'app.multiplayer.status.error',
		defaultMessage: 'Error',
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
})

const tab = ref<'host' | 'join'>('host')
const playerName = ref('')
const roomCodeInput = ref('')
const errorMessage = ref('')
const state = ref<any>(null)
const platformKey = ref('')
let pollInterval: ReturnType<typeof setInterval> | null = null

const statusText = computed(() => {
	if (!state.value) return formatMessage(messages.notRunning)
	const statusMap: Record<string, any> = {
		idle: messages.statusIdle,
		starting: messages.statusStarting,
		host_scanning: messages.statusHostScanning,
		host_ready: messages.statusHostReady,
		guest_connecting: messages.statusGuestConnecting,
		guest_ready: messages.statusGuestReady,
		error: messages.statusError,
		downloading: messages.statusDownloading,
	}
	return formatMessage(statusMap[state.value.status] ?? messages.notRunning)
})

const isRunning = computed(() => {
	return state.value && state.value.status !== 'idle' && state.value.status !== 'error' && state.value.status !== 'downloading'
})

const isHosting = computed(() => {
	return state.value && (state.value.status === 'host_scanning' || state.value.status === 'host_ready')
})

const isConnected = computed(() => {
	return state.value && (state.value.status === 'host_ready' || state.value.status === 'guest_ready')
})

const playerCount = computed(() => {
	return state.value?.players?.length ?? 0
})

const binaryPathHint = computed(() => {
	const name = platformKey.value?.includes('windows') ? 'terracotta.exe' : 'terracotta'
	return `<launcher_dir>/terracotta/${name}`
})

async function pollState() {
	try {
		const result = await invoke('plugin:terracotta|terracotta_get_state')
		state.value = result
		errorMessage.value = ''
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function startTerracotta() {
	errorMessage.value = ''
	try {
		await invoke('plugin:terracotta|terracotta_start', { autoDownload: true })
		pollInterval = setInterval(pollState, 1000)
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function stopTerracotta() {
	try {
		await invoke('plugin:terracotta|terracotta_stop')
		if (pollInterval) {
			clearInterval(pollInterval)
			pollInterval = null
		}
		state.value = null
		errorMessage.value = ''
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function hostGame() {
	errorMessage.value = ''
	if (!playerName.value.trim()) {
		errorMessage.value = 'Please enter a player name'
		return
	}
	if (!state.value?.http_port) {
		await startTerracotta()
	}
	try {
		await invoke('plugin:terracotta|terracotta_host', {
			playerName: playerName.value.trim(),
			roomCode: null,
		})
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function joinGame() {
	errorMessage.value = ''
	if (!playerName.value.trim()) {
		errorMessage.value = 'Please enter a player name'
		return
	}
	if (!roomCodeInput.value.trim()) {
		errorMessage.value = 'Please enter a room code'
		return
	}
	if (!state.value?.http_port) {
		await startTerracotta()
	}
	try {
		const parsed = await invoke('plugin:terracotta|terracotta_parse_room_code', {
			roomCode: roomCodeInput.value.trim(),
		})
		await invoke('plugin:terracotta|terracotta_join', {
			playerName: playerName.value.trim(),
			roomCode: parsed,
		})
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function resetState() {
	try {
		await invoke('plugin:terracotta|terracotta_reset')
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

async function downloadTerracotta() {
	errorMessage.value = ''
	try {
		await invoke('plugin:terracotta|terracotta_download', { version: null as any })
		pollInterval = setInterval(pollState, 1000)
	} catch (e: any) {
		errorMessage.value = String(e)
	}
}

onMounted(async () => {
	await pollState()
	try {
		platformKey.value = await invoke('plugin:terracotta|terracotta_get_platform_key')
	} catch (e: any) {
		platformKey.value = 'unknown'
	}
})

onUnmounted(() => {
	if (pollInterval) {
		clearInterval(pollInterval)
	}
})
</script>

<template>
	<div class="p-6 flex flex-col gap-6 max-w-2xl mx-auto w-full">
		<h1 class="text-2xl font-bold">{{ formatMessage(messages.title) }}</h1>

		<div v-if="isConnected" class="bg-bg-raised rounded-xl p-6 border border-surface-5">
			<div class="flex items-center gap-3 mb-4">
				<div class="w-3 h-3 rounded-full bg-green-500 flex-shrink-0" />
				<div>
					<div class="font-semibold text-lg">{{ statusText }}</div>
					<div v-if="isHosting" class="text-sm text-secondary mt-1">
						{{ formatMessage(messages.shareCode) }}
					</div>
				</div>
			</div>
			<div v-if="isHosting && state?.room_code" class="mb-4">
				<div class="flex items-center gap-2 bg-surface-5 rounded-lg p-3">
					<code class="text-lg font-mono select-all">{{ state.room_code }}</code>
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
						<span>{{ player.name }}</span>
						<span class="text-secondary text-xs px-1.5 py-0.5 bg-surface-10 rounded">
							{{ player.kind === 'HOST' ? formatMessage(messages.hostLabel) : formatMessage(messages.guestLabel) }}
						</span>
					</div>
				</div>
				<div v-else class="text-sm text-secondary">
					{{ formatMessage(messages.statusGuestConnecting) }}
				</div>
			</div>
			<div class="flex gap-2">
				<Button color="danger" @click="stopTerracotta">
					{{ formatMessage(messages.stop) }}
				</Button>
				<Button v-if="isHosting" @click="resetState">
					{{ formatMessage(messages.stop) }}
				</Button>
			</div>
		</div>

		<div v-else-if="state?.http_port" class="bg-bg-raised rounded-xl p-6 border border-surface-5">
			<div class="flex items-center gap-3 mb-4">
				<div
					class="w-3 h-3 rounded-full flex-shrink-0"
					:class="{
						'bg-yellow-500 animate-pulse': state.status === 'starting' || state.status === 'host_scanning' || state.status === 'guest_connecting' || state.status === 'downloading',
						'bg-red-500': state.status === 'error',
					}"
				/>
				<div v-if="state.status === 'downloading' && state.download_progress != null" class="mb-3">
					<div class="h-2 bg-surface-5 rounded-full overflow-hidden">
						<div
							class="h-full bg-brand rounded-full transition-all duration-300"
							:style="{ width: state.download_progress + '%' }"
						/>
					</div>
					<div class="text-xs text-secondary mt-1">{{ state.download_progress }}%</div>
				</div>
				<div class="font-semibold">{{ statusText }}</div>
			</div>
			<div v-if="errorMessage" class="text-red-500 text-sm mb-4">{{ errorMessage }}</div>
			<div v-if="state.status !== 'error'" class="text-sm text-secondary">
				{{ tab === 'host' ? formatMessage(messages.hostDescription) : formatMessage(messages.joinDescription) }}
			</div>
			<div class="mt-4 flex gap-2">
				<Button @click="stopTerracotta">
					{{ formatMessage(messages.stop) }}
				</Button>
			</div>
		</div>

		<div v-else class="flex flex-col gap-6">
			<div
				v-if="!state?.http_port"
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
				<Button class="mt-3" size="small" @click="downloadTerracotta">
					{{ formatMessage(messages.downloadTerracotta) }}
				</Button>
			</div>

			<div class="bg-bg-raised rounded-xl border border-surface-5 overflow-hidden">
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

					<div
						v-if="tab === 'host'"
						class="text-sm text-secondary"
					>
						{{ formatMessage(messages.hostDescription) }}
					</div>
					<div
						v-if="tab === 'join'"
						class="text-sm text-secondary"
					>
						{{ formatMessage(messages.joinDescription) }}
					</div>

					<div v-if="errorMessage" class="text-red-500 text-sm">{{ errorMessage }}</div>

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
		</div>
	</div>
</template>
