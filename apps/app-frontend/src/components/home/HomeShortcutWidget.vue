<script setup lang="ts">
import { defineMessages, GAME_MODES, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import type { HomeWidgetPlacement } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import Instance from '@/components/ui/Instance.vue'
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
	<div class="flex h-full min-h-0 flex-col justify-center">
		<div
			v-if="missing"
			class="flex h-full min-h-0 flex-col items-center justify-center gap-2 text-center"
		>
			<strong class="max-w-full truncate text-contrast">{{
				placement.target?.fallbackLabel
			}}</strong>
			<span class="text-sm text-secondary">{{ formatMessage(messages.unavailable) }}</span>
		</div>
		<Instance
			v-else-if="placement.kind === 'instance' && instance"
			:instance="instance"
			compact
			flat
			:playing="runningInstanceIds.includes(instance.id)"
		/>
		<WorldItem
			v-else-if="world && instance"
			:world="world"
			flat
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
