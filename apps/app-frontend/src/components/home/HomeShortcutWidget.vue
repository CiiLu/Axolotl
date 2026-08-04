<script setup lang="ts">
import {
	BoxIcon,
	GameIcon,
	IssuesIcon,
	NoSignalIcon,
	PlayIcon,
	ServerIcon,
	SignalIcon,
	SpinnerIcon,
	StopCircleIcon,
	TimerIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	GAME_MODES,
	injectNotificationManager,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import dayjs from 'dayjs'
import { computed, ref, watch } from 'vue'

import type { HomeWidgetPlacement, HomeWidgetSize } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { kill, run } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import {
	getWorldIdentifier,
	hasServerQuickPlaySupport,
	hasWorldQuickPlaySupport,
	start_join_server,
	start_join_singleplayer_world,
	type World,
} from '@/helpers/worlds'
import { handleSevereError } from '@/store/error'

const props = defineProps<{
	placement: HomeWidgetPlacement
	instances: GameInstance[]
	dashboardSize: HomeWidgetSize
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatRelativeTime = useRelativeTime()
const handleMinecraftLaunchError = useMinecraftLaunchError()
const runtime = useHomeDashboardRuntime()
const { gameVersions, runningInstanceIds } = runtime
const world = ref<World | null>(null)
const starting = ref(false)
const loadingTarget = ref(false)

const messages = defineMessages({
	unavailable: {
		id: 'app.home.widgets.unavailable',
		defaultMessage: 'Content unavailable',
	},
	played: { id: 'app.instance.played', defaultMessage: 'Played {time}' },
	neverPlayed: { id: 'app.instance.never-played', defaultMessage: 'Never played' },
	instance: { id: 'app.home.shortcut.kind.instance', defaultMessage: 'Instance' },
	world: { id: 'app.home.shortcut.kind.world', defaultMessage: 'World' },
	server: { id: 'app.home.shortcut.kind.server', defaultMessage: 'Server' },
	offline: { id: 'app.home.shortcut.server.offline', defaultMessage: 'Server offline' },
	playersOnline: {
		id: 'app.home.shortcut.server.players-online',
		defaultMessage: '{count} online',
	},
	hardcore: { id: 'instance.worlds.hardcore', defaultMessage: 'Hardcore mode' },
	noServerQuickPlay: {
		id: 'instance.worlds.no_server_quick_play',
		defaultMessage: 'Direct server join is unavailable for this Minecraft version.',
	},
	noWorldQuickPlay: {
		id: 'instance.worlds.no_singleplayer_quick_play',
		defaultMessage: 'Direct world launch is unavailable for this Minecraft version.',
	},
})

const instance = computed(() =>
	props.instances.find((candidate) => candidate.id === props.placement.target?.instanceId),
)
const missing = computed(
	() => !instance.value || (props.placement.kind !== 'instance' && !world.value),
)
const serverData = computed(() =>
	instance.value && world.value?.type === 'server'
		? runtime.getServerData(instance.value.id, world.value.address)
		: undefined,
)
const isRunning = computed(() =>
	instance.value ? runningInstanceIds.value.includes(instance.value.id) : false,
)
const versionLabel = computed(() => {
	if (!instance.value) return ''
	const loader = instance.value.loader === 'vanilla' ? 'Minecraft' : instance.value.loader
	return `${loader} ${instance.value.game_version}`
})
const lastPlayedLabel = computed(() => {
	const lastPlayed = world.value?.last_played ?? instance.value?.last_played
	return lastPlayed
		? formatMessage(messages.played, {
				time: formatRelativeTime(dayjs(lastPlayed).toISOString()),
			})
		: formatMessage(messages.neverPlayed)
})
const shortcutTitle = computed(
	() =>
		(props.placement.kind === 'instance' ? instance.value?.name : world.value?.name) ??
		props.placement.target?.fallbackLabel ??
		'',
)
const shortcutRoute = computed(() => {
	if (!instance.value) return '/'
	if (!world.value) return `/instance/${encodeURIComponent(instance.value.id)}`
	return `/instance/${encodeURIComponent(instance.value.id)}/worlds?highlight=${encodeURIComponent(getWorldIdentifier(world.value))}`
})
const shortcutIcon = computed(() => {
	if (!world.value) return undefined
	return world.value.type === 'server'
		? (serverData.value?.status?.favicon ?? world.value.icon)
		: world.value.icon
})
const kindLabel = computed(() =>
	formatMessage(
		props.placement.kind === 'instance'
			? messages.instance
			: props.placement.kind === 'world'
				? messages.world
				: messages.server,
	),
)
const kindIcon = computed(() =>
	props.placement.kind === 'instance'
		? BoxIcon
		: props.placement.kind === 'world'
			? GameIcon
			: ServerIcon,
)
const primaryLabel = computed(() => {
	if (!world.value) return versionLabel.value
	if (world.value.type === 'singleplayer') {
		return world.value.hardcore
			? formatMessage(messages.hardcore)
			: formatMessage(GAME_MODES[world.value.game_mode].message)
	}
	if (serverData.value?.refreshing) return formatMessage(commonMessages.loadingLabel)
	if (!serverData.value?.status) return formatMessage(messages.offline)
	return formatMessage(messages.playersOnline, {
		count: serverData.value.status.players?.online ?? 0,
	})
})
const secondaryLabel = computed(() => {
	if (world.value?.type === 'server') return world.value.address
	if (world.value) return `${instance.value?.name ?? ''} · ${lastPlayedLabel.value}`
	return lastPlayedLabel.value
})
const statusIcon = computed(() => {
	if (world.value?.type !== 'server') return world.value ? GameIcon : TimerIcon
	return serverData.value?.status ? SignalIcon : NoSignalIcon
})
const supportsQuickPlay = computed(() => {
	if (!world.value || !instance.value) return true
	return world.value.type === 'server'
		? hasServerQuickPlaySupport(gameVersions.value, instance.value.game_version)
		: hasWorldQuickPlaySupport(gameVersions.value, instance.value.game_version)
})
const playTooltip = computed(() => {
	if (supportsQuickPlay.value) return formatMessage(commonMessages.playButton)
	return formatMessage(
		world.value?.type === 'server' ? messages.noServerQuickPlay : messages.noWorldQuickPlay,
	)
})

async function refreshTarget(force = false) {
	world.value = null
	const target = props.placement.target
	if (!target || props.placement.kind === 'instance' || !instance.value) return

	loadingTarget.value = true
	try {
		const available = await runtime.getInstanceWorlds(target.instanceId, force)
		world.value =
			available.find((candidate) =>
				candidate.type === 'server'
					? props.placement.kind === 'server' && candidate.address === target.address
					: props.placement.kind === 'world' && candidate.path === target.path,
			) ?? null

		if (world.value?.type === 'server') {
			await runtime.refreshServer(target.instanceId, world.value.address, force)
		}
	} finally {
		loadingTarget.value = false
	}
}

async function playInstance(targetInstance: GameInstance) {
	starting.value = true
	try {
		await run(targetInstance.id)
		trackEvent('InstanceStart', {
			loader: targetInstance.loader,
			game_version: targetInstance.game_version,
			source: 'HomeInstanceWidget',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: targetInstance.id,
			instance_name: targetInstance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: targetInstance.id })
	} finally {
		starting.value = false
	}
}

async function playWorld() {
	if (!instance.value || !world.value) return
	starting.value = true
	try {
		if (world.value.type === 'server') {
			await start_join_server(instance.value.id, world.value.address)
		} else {
			await start_join_singleplayer_world(instance.value.id, world.value.path)
		}
		trackEvent('InstanceStart', {
			loader: instance.value.loader,
			game_version: instance.value.game_version,
			source: 'HomeShortcutWidget',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: instance.value.id,
			instance_name: instance.value.name,
		})
		if (!handled) handleSevereError(error, { instanceId: instance.value.id })
	} finally {
		starting.value = false
	}
}

async function playShortcut() {
	if (!instance.value) return
	if (world.value) await playWorld()
	else await playInstance(instance.value)
}

async function stopInstance() {
	if (!instance.value) return
	await kill(instance.value.id).catch(handleError)
}

watch(
	() => [props.placement, props.instances] as const,
	() => refreshTarget(),
	{
		immediate: true,
		deep: true,
	},
)
</script>

<template>
	<div class="home-shortcut-widget" :data-size="dashboardSize" :data-kind="placement.kind">
		<div v-if="loadingTarget" class="home-shortcut-missing">
			<SpinnerIcon class="size-6 animate-spin text-secondary" aria-hidden="true" />
			<span class="text-sm text-secondary">{{ formatMessage(commonMessages.loadingLabel) }}</span>
		</div>
		<div v-else-if="missing" class="home-shortcut-missing">
			<IssuesIcon class="size-6 text-secondary" aria-hidden="true" />
			<strong class="max-w-full truncate text-contrast">{{
				placement.target?.fallbackLabel
			}}</strong>
			<span class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
		</div>
		<div v-else class="home-shortcut-card">
			<router-link class="home-shortcut-visual" :to="shortcutRoute" tabindex="-1">
				<component :is="kindIcon" class="home-shortcut-watermark" aria-hidden="true" />
				<Avatar
					v-if="shortcutIcon"
					class="home-shortcut-icon"
					:src="shortcutIcon"
					:size="dashboardSize === '2x1' ? '72px' : '44px'"
				/>
				<InstanceIcon
					v-else-if="instance"
					class="home-shortcut-icon"
					:icon-path="instance.icon_path"
					:instance-id="instance.id"
					:size="dashboardSize === '2x1' ? '72px' : '44px'"
				/>
			</router-link>

			<div class="home-shortcut-body">
				<router-link class="home-shortcut-copy" :to="shortcutRoute">
					<span class="home-shortcut-kind">
						<component :is="kindIcon" aria-hidden="true" />
						{{ kindLabel }}
					</span>
					<strong class="home-shortcut-title">{{ shortcutTitle }}</strong>
					<span class="home-shortcut-meta home-shortcut-primary">
						<SpinnerIcon
							v-if="world?.type === 'server' && serverData?.refreshing"
							class="animate-spin"
							aria-hidden="true"
						/>
						<component :is="statusIcon" v-else aria-hidden="true" />
						{{ primaryLabel }}
					</span>
					<span class="home-shortcut-meta home-shortcut-secondary">
						<TimerIcon v-if="world?.type !== 'server'" aria-hidden="true" />
						<ServerIcon v-else aria-hidden="true" />
						{{ secondaryLabel }}
					</span>
				</router-link>

				<div class="home-shortcut-action">
					<ButtonStyled v-if="isRunning" circular size="small" color="red">
						<button v-tooltip="formatMessage(commonMessages.stopButton)" @click="stopInstance">
							<StopCircleIcon />
						</button>
					</ButtonStyled>
					<ButtonStyled v-else circular size="small" color="brand">
						<button
							v-tooltip="playTooltip"
							:disabled="starting || !supportsQuickPlay"
							@click="playShortcut"
						>
							<SpinnerIcon v-if="starting" class="animate-spin" />
							<PlayIcon v-else class="translate-x-px" />
						</button>
					</ButtonStyled>
				</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
.home-shortcut-widget,
.home-shortcut-card {
	min-width: 0;
	min-height: 0;
	height: 100%;
}

.home-shortcut-card {
	display: grid;
	overflow: hidden;
}

.home-shortcut-visual {
	position: relative;
	display: flex;
	min-width: 0;
	min-height: 0;
	align-items: center;
	justify-content: center;
	overflow: hidden;
	background: var(--color-button-bg);
	color: var(--color-secondary);
	text-decoration: none;
}

.home-shortcut-watermark {
	position: absolute;
	right: 0.75rem;
	bottom: -0.75rem;
	width: 4.5rem;
	height: 4.5rem;
	opacity: 0.08;
}

.home-shortcut-icon {
	position: relative;
	z-index: 1;
	flex: 0 0 auto;
	box-shadow: var(--shadow-card);
}

.home-shortcut-body {
	position: relative;
	display: flex;
	min-width: 0;
	min-height: 0;
	align-items: stretch;
}

.home-shortcut-copy {
	display: flex;
	min-width: 0;
	flex: 1;
	flex-direction: column;
	color: inherit;
	text-decoration: none;
}

.home-shortcut-copy:focus-visible {
	border-radius: 6px;
	outline: 4px solid var(--color-brand-shadow);
	outline-offset: 2px;
}

.home-shortcut-kind {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.3rem;
	color: var(--color-secondary);
	font-size: 0.6875rem;
	font-weight: 700;
	line-height: 1;
}

.home-shortcut-kind svg,
.home-shortcut-meta svg {
	width: 0.8rem;
	height: 0.8rem;
	flex: 0 0 auto;
}

.home-shortcut-title {
	min-width: 0;
	overflow: hidden;
	color: var(--color-contrast);
	font-weight: 750;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-shortcut-copy:hover .home-shortcut-title {
	text-decoration: underline;
}

.home-shortcut-meta {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.35rem;
	overflow: hidden;
	color: var(--color-secondary);
	font-size: 0.75rem;
	font-weight: 600;
	line-height: 1.2;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-shortcut-action {
	position: absolute;
	right: 0.75rem;
	bottom: 0.75rem;
	z-index: 2;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-card {
	grid-template-rows: 3.75rem minmax(0, 1fr);
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-visual {
	justify-content: flex-start;
	padding: 0 0.875rem;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-watermark {
	right: 0.5rem;
	bottom: -1.25rem;
	width: 4rem;
	height: 4rem;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-body {
	padding: 0.625rem 0.75rem 0.75rem;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-copy {
	padding-right: 2.5rem;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-kind {
	display: none;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-title {
	font-size: 0.9375rem;
	line-height: 1.2;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-primary {
	margin-top: 0.3rem;
}

.home-shortcut-widget[data-size='1x1'] .home-shortcut-secondary {
	display: none;
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-card {
	grid-template-columns: minmax(8.5rem, 0.8fr) minmax(0, 1.65fr);
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-body {
	padding: 1rem;
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-copy {
	justify-content: center;
	padding-right: 3rem;
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-title {
	margin-top: 0.4rem;
	font-size: 1.125rem;
	line-height: 1.25;
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-primary {
	margin-top: 0.65rem;
}

.home-shortcut-widget[data-size='2x1'] .home-shortcut-secondary {
	margin-top: 0.3rem;
}

.home-shortcut-missing {
	display: flex;
	min-width: 0;
	min-height: 0;
	height: 100%;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	gap: 0.5rem;
	padding: 1rem;
	box-sizing: border-box;
	text-align: center;
}
</style>
