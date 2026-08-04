<script setup lang="ts">
import {
	GameIcon,
	IssuesIcon,
	PlayIcon,
	SpinnerIcon,
	StopCircleIcon,
	TimerIcon,
} from '@modrinth/assets'
import {
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
import WorldItem from '@/components/ui/world/WorldItem.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { kill, run } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import {
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

const messages = defineMessages({
	unavailable: {
		id: 'app.home.widgets.unavailable',
		defaultMessage: 'Content unavailable',
	},
	played: { id: 'app.instance.played', defaultMessage: 'Played {time}' },
	neverPlayed: { id: 'app.instance.never-played', defaultMessage: 'Never played' },
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
const protocolVersion = computed(() =>
	instance.value ? (runtime.getProtocolVersion(instance.value.id) ?? null) : null,
)
const versionLabel = computed(() => {
	if (!instance.value) return ''
	const loader = instance.value.loader === 'vanilla' ? 'Minecraft' : instance.value.loader
	return `${loader} ${instance.value.game_version}`
})
const lastPlayedLabel = computed(() =>
	instance.value?.last_played
		? formatMessage(messages.played, {
				time: formatRelativeTime(dayjs(instance.value.last_played).toISOString()),
			})
		: formatMessage(messages.neverPlayed),
)

async function refreshTarget(force = false) {
	world.value = null
	const target = props.placement.target
	if (!target || props.placement.kind === 'instance' || !instance.value) return

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
		<div v-if="missing" class="home-shortcut-missing">
			<IssuesIcon class="size-6 text-secondary" aria-hidden="true" />
			<strong class="max-w-full truncate text-contrast">{{
				placement.target?.fallbackLabel
			}}</strong>
			<span class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
		</div>
		<div v-else-if="placement.kind === 'instance' && instance" class="home-instance-shortcut">
			<router-link
				class="home-instance-shortcut-main"
				:to="`/instance/${encodeURIComponent(instance.id)}`"
			>
				<InstanceIcon
					:icon-path="instance.icon_path"
					:instance-id="instance.id"
					:alt="instance.name"
					:size="dashboardSize === '2x1' ? '64px' : '52px'"
				/>
				<span class="home-instance-shortcut-copy">
					<strong>{{ instance.name }}</strong>
					<span><GameIcon aria-hidden="true" /> {{ versionLabel }}</span>
					<span><TimerIcon aria-hidden="true" /> {{ lastPlayedLabel }}</span>
				</span>
			</router-link>
			<ButtonStyled v-if="runningInstanceIds.includes(instance.id)" circular color="red">
				<button v-tooltip="formatMessage(commonMessages.stopButton)" @click="stopInstance">
					<StopCircleIcon />
				</button>
			</ButtonStyled>
			<ButtonStyled v-else circular color="brand">
				<button
					v-tooltip="formatMessage(commonMessages.playButton)"
					:disabled="starting"
					@click="playInstance(instance)"
				>
					<SpinnerIcon v-if="starting" class="animate-spin" />
					<PlayIcon v-else class="translate-x-px" />
				</button>
			</ButtonStyled>
		</div>
		<WorldItem
			v-else-if="world && instance"
			:world="world"
			flat
			:dashboard-density="dashboardSize === '1x1' ? 'compact' : 'comfortable'"
			:playing-instance="runningInstanceIds.includes(instance.id)"
			:starting-instance="starting"
			:supports-server-quick-play="
				world.type === 'server' && hasServerQuickPlaySupport(gameVersions, instance.game_version)
			"
			:supports-world-quick-play="
				world.type === 'singleplayer' &&
				hasWorldQuickPlaySupport(gameVersions, instance.game_version)
			"
			:current-protocol="protocolVersion"
			:refreshing="world.type === 'server' ? serverData?.refreshing : undefined"
			:server-status="world.type === 'server' ? serverData?.status : undefined"
			:rendered-motd="world.type === 'server' ? serverData?.renderedMotd : undefined"
			:game-mode="world.type === 'singleplayer' ? GAME_MODES[world.game_mode] : undefined"
			:instance-id="instance.id"
			:instance-name="instance.name"
			:instance-icon="instance.icon_path"
			:shortcut-instance-id="instance.id"
			@play="playWorld"
			@play-instance="playInstance(instance)"
			@stop="stopInstance"
			@refresh="refreshTarget(true)"
		/>
	</div>
</template>

<style scoped>
.home-shortcut-widget {
	display: flex;
	min-width: 0;
	min-height: 0;
	height: 100%;
	flex-direction: column;
	justify-content: center;
}

.home-shortcut-widget[data-size='2x1'] > :deep(*) {
	width: 100%;
}

.home-instance-shortcut {
	display: grid;
	min-width: 0;
	height: 100%;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: center;
	gap: 1rem;
}

.home-instance-shortcut-main {
	display: grid;
	min-width: 0;
	grid-template-columns: auto minmax(0, 1fr);
	align-items: center;
	gap: 0.875rem;
	color: inherit;
	text-decoration: none;
}

.home-instance-shortcut-main:focus-visible {
	border-radius: var(--radius-lg);
	outline: 4px solid var(--color-brand-shadow);
	outline-offset: 3px;
}

.home-instance-shortcut-copy {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.25rem;
}

.home-instance-shortcut-copy strong {
	overflow: hidden;
	color: var(--color-contrast);
	font-size: 1rem;
	font-weight: 700;
	line-height: 1.25;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-instance-shortcut-copy span {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.3rem;
	overflow: hidden;
	color: var(--color-secondary);
	font-size: 0.75rem;
	font-weight: 600;
	line-height: 1.2;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-instance-shortcut-copy svg {
	width: 0.875rem;
	height: 0.875rem;
	flex: 0 0 auto;
}

.home-shortcut-widget[data-size='2x1'] .home-instance-shortcut {
	gap: 1.5rem;
	padding: 0 0.75rem;
}

.home-shortcut-widget[data-size='2x1'] .home-instance-shortcut-copy strong {
	font-size: 1.125rem;
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
	text-align: center;
}
</style>
