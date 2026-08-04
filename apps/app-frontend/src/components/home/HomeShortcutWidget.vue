<script setup lang="ts">
import { IssuesIcon } from '@modrinth/assets'
import { defineMessages, GAME_MODES, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import type { HomeWidgetPlacement, HomeWidgetSize } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import HomeInstanceCard from '@/components/home/HomeInstanceCard.vue'
import WorldItem from '@/components/ui/world/WorldItem.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { kill, run, set_pinned } from '@/helpers/instance'
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

async function updatePinned(targetInstance: GameInstance, pinned: boolean) {
	await set_pinned(targetInstance.id, pinned).catch(handleError)
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
			<span class="home-shortcut-missing-icon"><IssuesIcon aria-hidden="true" /></span>
			<strong class="max-w-full truncate text-contrast">{{
				placement.target?.fallbackLabel
			}}</strong>
			<span class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
		</div>
		<HomeInstanceCard
			v-else-if="placement.kind === 'instance' && instance"
			:instance="instance"
			:pinned="!!instance.pinned_at"
			:playing="runningInstanceIds.includes(instance.id)"
			:layout="dashboardSize === '1x1' ? 'spotlight' : 'row'"
			@pinned-change="updatePinned"
		/>
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

.home-shortcut-widget[data-size='2x1'] {
	justify-content: stretch;
}

.home-shortcut-widget[data-size='2x1'] > :deep(*) {
	width: 100%;
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

.home-shortcut-missing-icon {
	display: inline-flex;
	width: 2.5rem;
	height: 2.5rem;
	align-items: center;
	justify-content: center;
	border-radius: 6px;
	background: color-mix(in srgb, var(--color-orange) 14%, transparent);
	color: var(--color-orange);
}

.home-shortcut-missing-icon svg {
	width: 1.25rem;
	height: 1.25rem;
}
</style>
