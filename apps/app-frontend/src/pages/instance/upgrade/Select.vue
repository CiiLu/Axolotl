<template>
	<section class="flex flex-col gap-6 py-2">
		<header class="flex flex-col gap-1">
			<h2 class="m-0 text-xl font-semibold text-contrast">
				{{ formatMessage(messages.title) }}
			</h2>
			<p class="m-0 max-w-2xl text-secondary">
				{{ formatMessage(messages.description) }}
			</p>
		</header>

		<div class="grid gap-4 md:grid-cols-2">
			<Card class="!m-0 p-4">
				<h3 class="m-0 text-sm font-semibold text-secondary">
					{{ formatMessage(messages.current) }}
				</h3>
				<p class="mb-1 mt-3 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.minecraftVersion, { version: instance.game_version }) }}
				</p>
				<p class="m-0 text-secondary">{{ currentLoaderLabel }}</p>
			</Card>

			<Card class="!m-0 p-4">
				<h3 class="m-0 text-sm font-semibold text-secondary">
					{{ formatMessage(messages.target) }}
				</h3>
				<label class="mb-2 mt-3 block text-sm font-medium text-contrast">
					{{ formatMessage(messages.minecraft) }}
				</label>
				<div v-if="gameVersionsQuery.isPending.value" class="text-sm text-secondary">
					{{ formatMessage(messages.loadingVersions) }}
				</div>
				<DropdownSelect
					v-else-if="targetVersions.length"
					v-model="selectedGameVersion"
					class="max-w-full"
					:name="formatMessage(messages.targetVersionInput)"
					:options="targetVersions"
					:disabled="flow.busy.value"
				/>
				<p v-else class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.noNewerRelease) }}
				</p>
				<p class="mb-0 mt-3 text-secondary">{{ formatLoaderLabel(instance.loader) }}</p>
			</Card>
		</div>

		<Admonition
			v-if="gameVersionsQuery.isError.value"
			type="critical"
			:header="formatMessage(messages.metadataErrorTitle)"
		>
			{{ formatMessage(messages.metadataErrorBody) }}
		</Admonition>
		<Admonition
			v-else-if="versionTargets && !versionTargets.currentFound"
			type="warning"
			:header="formatMessage(messages.currentVersionMissingTitle)"
		>
			{{ formatMessage(messages.currentVersionMissingBody) }}
		</Admonition>
		<Admonition
			v-if="flow.error.value"
			type="critical"
			:header="formatMessage(messages.planningErrorTitle)"
		>
			{{ errorMessage(flow.error.value) }}
		</Admonition>

		<div v-if="flow.busy.value" class="flex items-center gap-2 text-secondary" role="status">
			<SpinnerIcon class="size-5 animate-spin" aria-hidden="true" />
			{{ formatMessage(messages.planningStatus, { count: snapshotItemCount }) }}
		</div>

	</section>
</template>

<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import {
	Admonition,
	Card,
	defineMessages,
	DropdownSelect,
	formatLoaderLabel,
	useVIntl,
} from '@modrinth/ui'
import type { GameVersionTag } from '@modrinth/utils'
import { useQuery } from '@tanstack/vue-query'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { loadInstanceContentData } from '@/helpers/instance-content'
import { plan_instance_upgrade } from '@/helpers/instance-upgrade'
import { get_game_versions } from '@/helpers/tags'

import { inferShaderRuntime, newerStableGameVersions } from './analysis'
import { useInstanceUpgradeFlow } from './flow'

const messages = defineMessages({
	title: { id: 'instance.upgrade.select.title', defaultMessage: 'Upgrade instance' },
	description: {
		id: 'instance.upgrade.select.description',
		defaultMessage: 'Choose which Minecraft version this instance should be upgraded to.',
	},
	current: { id: 'instance.upgrade.select.current', defaultMessage: 'Current' },
	target: { id: 'instance.upgrade.select.target', defaultMessage: 'Target' },
	minecraft: { id: 'instance.upgrade.select.minecraft', defaultMessage: 'Minecraft version' },
	minecraftVersion: {
		id: 'instance.upgrade.select.minecraft-version',
		defaultMessage: 'Minecraft {version}',
	},
	targetVersionInput: {
		id: 'instance.upgrade.select.target-version-input',
		defaultMessage: 'Target Minecraft version',
	},
	loadingVersions: {
		id: 'instance.upgrade.select.loading-versions',
		defaultMessage: 'Loading Minecraft versions…',
	},
	noNewerRelease: {
		id: 'instance.upgrade.select.no-newer-release',
		defaultMessage: 'No newer stable Minecraft release is available.',
	},
	metadataErrorTitle: {
		id: 'instance.upgrade.select.metadata-error-title',
		defaultMessage: 'Minecraft versions could not be loaded',
	},
	metadataErrorBody: {
		id: 'instance.upgrade.select.metadata-error-body',
		defaultMessage: 'Check your connection and try again.',
	},
	currentVersionMissingTitle: {
		id: 'instance.upgrade.select.current-version-missing-title',
		defaultMessage: 'Current version not found in metadata',
	},
	currentVersionMissingBody: {
		id: 'instance.upgrade.select.current-version-missing-body',
		defaultMessage: 'Stable releases are shown without guessing their numeric order.',
	},
	planningErrorTitle: {
		id: 'instance.upgrade.select.planning-error-title',
		defaultMessage: 'Compatibility analysis failed',
	},
	planningStatus: {
		id: 'instance.upgrade.select.planning-status',
		defaultMessage: 'Analyzing compatibility for {count} content items…',
	},
	checkCompatibility: {
		id: 'instance.upgrade.select.check-compatibility',
		defaultMessage: 'Check compatibility',
	},
})

const flow = useInstanceUpgradeFlow()
const router = useRouter()
const { formatMessage } = useVIntl()
const instance = computed(() => flow.instance.value)
const selectedGameVersion = ref<string | null>(flow.targetEnvironment.value?.gameVersion ?? null)

const gameVersionsQuery = useQuery({
	queryKey: ['instance-upgrade', 'game-versions'],
	queryFn: () => get_game_versions() as Promise<GameVersionTag[]>,
})
const contentDataQuery = useQuery({
	queryKey: computed(() => ['instance-upgrade', 'content-data', flow.instanceId.value]),
	queryFn: () => loadInstanceContentData(flow.instanceId.value),
	staleTime: Number.POSITIVE_INFINITY,
})

const versionTargets = computed(() => {
	if (!gameVersionsQuery.data.value) return null
	return newerStableGameVersions(gameVersionsQuery.data.value, instance.value.game_version)
})
const targetVersions = computed(() => versionTargets.value?.versions ?? [])
const currentLoaderLabel = computed(() => {
	const loader = formatLoaderLabel(instance.value.loader)
	return instance.value.loader_version ? `${loader} ${instance.value.loader_version}` : loader
})
const snapshotItemCount = computed(() => contentDataQuery.data.value?.snapshot.items.length ?? 0)
const canPlan = computed(
	() =>
		selectedGameVersion.value !== null &&
		targetVersions.value.includes(selectedGameVersion.value) &&
		!flow.busy.value &&
		!gameVersionsQuery.isError.value,
)

watch(
	versionTargets,
	(targets) => {
		if (!targets) return
		if (selectedGameVersion.value && targets.versions.includes(selectedGameVersion.value)) {
			return
		}
		selectedGameVersion.value = targets.versions[0] ?? null
	},
	{ immediate: true },
)

watch(selectedGameVersion, (version) => {
	if (flow.plan.value && version !== flow.plan.value.targetEnvironment.gameVersion) {
		flow.clearPlan()
	}
	flow.error.value = null
})

function registerControls() {
	flow.registerStepControls({
		canNext: canPlan,
		busy: flow.busy,
		nextLabel: formatMessage(messages.checkCompatibility),
		onNext: startPlanning,
		onBack: () => router.push(`/instance/${encodeURIComponent(flow.instanceId.value)}`),
	})
}
onMounted(registerControls)
watch([canPlan, () => flow.busy.value], registerControls)
onBeforeUnmount(() => flow.registerStepControls(null))

function errorMessage(error: unknown): string {
	if (error instanceof Error) return error.message
	if (typeof error === 'string') return error
	if (typeof error === 'object' && error && 'message' in error) return String(error.message)
	return String(error)
}

async function startPlanning() {
	if (!canPlan.value || !selectedGameVersion.value) return

	const targetEnvironment = {
		gameVersion: selectedGameVersion.value,
		modLoader: instance.value.loader,
		modLoaderVersion: null,
		shaderRuntime: inferShaderRuntime(instance.value, contentDataQuery.data.value?.snapshot),
	}
	flow.clearPlan()
	flow.setTargetEnvironment(targetEnvironment)
	flow.error.value = null
	flow.busy.value = true
	try {
		const plan = await plan_instance_upgrade(flow.instanceId.value, targetEnvironment)
		flow.setPlan(plan)
		await router.push(
			`/instance/${encodeURIComponent(flow.instanceId.value)}/upgrade/compatibility`,
		)
	} catch (error) {
		flow.setPlan(null)
		flow.error.value = error
	} finally {
		flow.busy.value = false
	}
}
</script>
