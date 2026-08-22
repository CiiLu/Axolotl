<template>
	<div class="mx-auto w-full" :class="wideCompatibilityLayout ? 'max-w-[96rem]' : 'max-w-5xl'">
		<RouterView v-if="instanceMatchesRoute" />
	</div>
	<UpgradeFlowFloatingBar v-if="instanceMatchesRoute" />
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, toRef, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

import type { GameInstance } from '@/helpers/types'
import { parkUpgradeFlow, restoreUpgradeFlow } from '@/helpers/upgrade-return-state'

import {
	isUpgradeRouteAvailable,
	provideInstanceUpgradeFlow,
	provideUpgradeFlow,
	type UpgradeRouteRequirement,
} from './flow'
import UpgradeFlowFloatingBar from './UpgradeFlowFloatingBar.vue'

const props = defineProps<{ instance: GameInstance }>()
const route = useRoute()
const router = useRouter()
const flow = provideInstanceUpgradeFlow(toRef(props, 'instance'))
provideUpgradeFlow(flow)
const routeInstanceId = computed(() =>
	Array.isArray(route.params.id) ? route.params.id[0] : route.params.id,
)
const instanceMatchesRoute = computed(() => routeInstanceId.value === props.instance.id)
const wideCompatibilityLayout = computed(() => route.path.endsWith('/upgrade/compatibility'))
const restoredSnapshot = restoreUpgradeFlow(props.instance.id, route.fullPath, flow.hydrate)

onMounted(async () => {
	if (restoredSnapshot?.scrollTop === undefined) return
	await nextTick()
	const viewport = document.querySelector('.app-viewport')
	if (viewport) viewport.scrollTop = restoredSnapshot.scrollTop
})

onBeforeRouteLeave((to) => {
	if (to.path.startsWith('/project/')) {
		parkUpgradeFlow({
			instanceId: props.instance.id,
			returnFullPath: route.fullPath,
			targetEnvironment: flow.targetEnvironment.value,
			plan: flow.plan.value,
			createFullBackup: flow.createFullBackup.value,
			directFullBackupPreference: flow.directFullBackupPreference.value,
			sharedUpgradeMode: flow.sharedUpgradeMode.value,
			activeJobId: flow.activeJobId.value,
			result: flow.result.value,
			initialBlockingPlanId: flow.initialBlockingPlanId.value,
			initialBlockingIssues: flow.initialBlockingIssues.value,
			customizeActiveStrategy: flow.customizeActiveStrategy.value,
			scrollTop: document.querySelector('.app-viewport')?.scrollTop,
		})
	}
})

function safeEntryPath(instanceId: string) {
	return `/instance/${encodeURIComponent(instanceId)}/upgrade`
}

function requirementFallback(instanceId: string, requirement: UpgradeRouteRequirement | undefined) {
	if (requirement === 'unblocked-plan' && flow.plan.value) {
		return `${safeEntryPath(instanceId)}/compatibility`
	}
	return safeEntryPath(instanceId)
}

watch(
	[() => route.fullPath, () => props.instance.id],
	async () => {
		if (!instanceMatchesRoute.value) return
		const requirement = route.meta.upgradeRequirement as UpgradeRouteRequirement | undefined
		if (!isUpgradeRouteAvailable(requirement, flow)) {
			await router.replace(requirementFallback(props.instance.id, requirement))
		}
	},
	{ immediate: true },
)
</script>
