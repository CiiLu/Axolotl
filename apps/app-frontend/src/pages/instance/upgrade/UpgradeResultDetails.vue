<template>
	<section class="flex flex-col gap-6 py-2">
		<header class="flex items-start gap-3">
			<CheckCircleIcon class="mt-0.5 size-8 shrink-0 text-green" aria-hidden="true" />
			<div>
				<h2 class="m-0 text-xl font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
				<p class="mb-0 mt-1 text-secondary">
					{{
						formatMessage(
							mode === 'copy_and_upgrade' ? messages.copyDescription : messages.directDescription,
						)
					}}
				</p>
			</div>
		</header>

		<div class="grid gap-3 md:grid-cols-2">
			<Card class="!m-0 p-4">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.environment) }}
				</h3>
				<div class="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
					<span class="text-secondary">{{ formatMessage(messages.minecraft) }}</span>
					<strong
						>{{ sourceEnvironment?.gameVersion ?? formatMessage(messages.unknown) }}
						<span aria-hidden="true">→</span>
						{{ targetEnvironment?.gameVersion ?? formatMessage(messages.unknown) }}</strong
					>
					<span class="text-secondary">{{ formatMessage(messages.loader) }}</span>
					<strong
						>{{ loaderLabel(sourceEnvironment) }} <span aria-hidden="true">→</span>
						{{ loaderLabel(actualTargetEnvironment) }}</strong
					>
				</div>
			</Card>
			<Card class="!m-0 p-4">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.metrics) }}
				</h3>
				<div class="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-3">
					<div v-for="metric in metrics" :key="metric.label">
						<div class="text-xl font-semibold text-contrast">{{ metric.value }}</div>
						<div class="text-xs text-secondary">{{ metric.label }}</div>
					</div>
				</div>
			</Card>
		</div>

		<div class="flex flex-wrap gap-2">
			<ButtonStyled color="brand"
				><button @click="router.push(`/instance/${encodeURIComponent(result.targetInstanceId)}`)">
					<ExternalIcon />{{ formatMessage(messages.openUpgraded) }}
				</button></ButtonStyled
			>
			<ButtonStyled v-if="mode === 'copy_and_upgrade'" type="outlined"
				><button @click="router.push(`/instance/${encodeURIComponent(result.sourceInstanceId)}`)">
					<ExternalIcon />{{ formatMessage(messages.openOriginal) }}
				</button></ButtonStyled
			>
			<ButtonStyled v-if="result.backupInstanceId" type="outlined"
				><button @click="router.push(`/instance/${encodeURIComponent(result.backupInstanceId!)}`)">
					<FolderOpenIcon />{{ formatMessage(messages.openBackup) }}
				</button></ButtonStyled
			>
		</div>

		<Card v-if="result.backupInstanceId" class="!m-0 p-4">
			<h3 class="m-0 text-base font-semibold text-contrast">
				{{ formatMessage(messages.backupTitle) }}
			</h3>
			<p class="mb-3 mt-1 text-sm text-secondary">
				{{ formatMessage(messages.backupDescription) }}
			</p>
		</Card>
		<Admonition
			v-else-if="mode === 'direct'"
			type="info"
			:header="formatMessage(messages.noBackupTitle)"
			>{{ formatMessage(messages.noBackupDescription) }}</Admonition
		>

		<Admonition
			v-if="result.externalChanges.length"
			type="warning"
			:header="formatMessage(messages.externalTitle)"
		>
			<p class="mb-2">{{ formatMessage(messages.externalDescription) }}</p>
			<ul class="m-0 list-disc pl-5">
				<li v-for="change in result.externalChanges" :key="`${change.kind}:${change.relativePath}`">
					<code>{{ change.relativePath }}</code> · {{ externalChangeLabel(change.kind) }}
				</li>
			</ul>
		</Admonition>
		<Admonition
			v-if="result.skippedDueToExternalConflict.length"
			type="warning"
			:header="formatMessage(messages.skippedTitle)"
		>
			<p class="mb-2">{{ formatMessage(messages.skippedDescription) }}</p>
			<ul class="m-0 list-disc pl-5">
				<li v-for="path in result.skippedDueToExternalConflict" :key="path">
					<code>{{ path }}</code>
				</li>
			</ul>
		</Admonition>
		<Card v-if="warningRows.length" class="!m-0 !p-0">
			<Accordion
				:open-by-default="warningsExpandedByDefault"
				button-class="flex w-full cursor-pointer items-center gap-2 border-0 bg-transparent p-4 text-left text-contrast hover:bg-surface-3"
				content-class="border-0 border-t border-solid border-divider"
			>
				<template #title>
					<TriangleAlertIcon class="size-5 shrink-0 text-orange" aria-hidden="true" />
					<strong>{{ formatMessage(messages.warningsTitle) }}</strong>
					<Badge color="orange" :type="String(warningRows.length)" />
				</template>
				<ul class="m-0 max-h-72 list-disc overflow-y-auto px-9 py-3">
					<li v-for="warning in warningRows" :key="warning.key" class="py-1">
						{{ warningLabel(warning) }}
					</li>
				</ul>
			</Accordion>
		</Card>

		<details class="rounded-md border border-solid border-surface-4 bg-surface-2 p-4">
			<summary class="cursor-pointer font-semibold text-contrast">
				{{ formatMessage(messages.detailsTitle) }}
			</summary>
			<div class="mt-3 flex flex-col gap-2">
				<div
					v-for="item in selectionDetails"
					:key="item.key"
					class="flex items-center justify-between gap-3 text-sm"
				>
					<div class="min-w-0">
						<RouterLink
							v-if="item.path"
							:to="item.path"
							class="font-medium text-contrast hover:text-brand hover:underline"
							>{{ item.title }}
							<ExternalIcon class="inline size-3" aria-hidden="true" /></RouterLink
						><span v-else class="text-contrast">{{ item.title }}</span>
						<div class="flex flex-wrap items-center gap-x-2 text-secondary">
							<UpgradeVersionChangelogPopout
								v-if="item.currentReleaseId"
								:label="item.current"
								:provider="item.provider"
								:project-id="item.projectId"
								:release-id="item.currentReleaseId"
							/><span v-else>{{ item.current }}</span
							><span v-if="item.target" aria-hidden="true">→</span
							><UpgradeVersionChangelogPopout
								v-if="item.targetReleaseId"
								:label="item.target"
								:provider="item.provider"
								:project-id="item.projectId"
								:release-id="item.targetReleaseId"
							/><span v-else-if="item.target">{{ item.target }}</span>
						</div>
					</div>
					<Badge
						:color="item.action === 'disable' ? 'gray' : item.action === 'keep' ? 'blue' : 'green'"
						:type="item.actionLabel"
					/>
				</div>
				<div
					v-for="change in result.solution.dependencyChanges"
					:key="`${change.provider}:${change.projectId}:${change.kind}:${change.targetReleaseId}`"
					class="flex items-center justify-between gap-3 border-0 border-t border-solid border-divider pt-2 text-sm"
				>
					<span class="text-contrast">{{ projectTitle(change.provider, change.projectId) }}</span
					><span class="text-secondary">{{ dependencyKindLabel(change.kind) }}</span>
				</div>
			</div>
		</details>
	</section>
</template>

<script setup lang="ts">
import { CheckCircleIcon, ExternalIcon, FolderOpenIcon, TriangleAlertIcon } from '@modrinth/assets'
import {
	Accordion,
	Admonition,
	Badge,
	ButtonStyled,
	Card,
	defineMessages,
	formatLoaderLabel,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { get_many as getInstances } from '@/helpers/instance'
import type {
	InstanceUpgradeExternalChangeKind,
	InstanceUpgradeTargetEnvironment,
} from '@/helpers/instance-upgrade'
import { shouldExpandUpgradeWarningsByDefault } from '@/helpers/post-upgrade-notice'
import { upgradeProjectPath } from '@/helpers/upgrade-return-state'
import {
	loadUpgradeProjectDisplayMetadata,
	loadUpgradeVersionDisplayMetadata,
	upgradeProjectDisplayCacheKey,
	type UpgradeProjectIdentity,
	type UpgradeReleaseIdentity,
	upgradeVersionDisplayLabel,
} from '@/helpers/upgrade-version-metadata'

import { summarizeUpgradeResult, upgradeResultMode } from './result'
import { upgradeResultWarningRows, type UpgradeWarningRow } from './upgrade-warning'
import UpgradeVersionChangelogPopout from './UpgradeVersionChangelogPopout.vue'

const messages = defineMessages({
	title: { id: 'instance.upgrade.result.title', defaultMessage: 'Upgrade complete' },
	directDescription: {
		id: 'instance.upgrade.result.direct-description',
		defaultMessage: 'This instance was upgraded successfully.',
	},
	copyDescription: {
		id: 'instance.upgrade.result.copy-description',
		defaultMessage:
			'An upgraded copy was created successfully. The original shared instance was left unchanged.',
	},
	environment: { id: 'instance.upgrade.result.environment', defaultMessage: 'Environment' },
	minecraft: { id: 'instance.upgrade.result.minecraft', defaultMessage: 'Minecraft' },
	loader: { id: 'instance.upgrade.result.loader', defaultMessage: 'Loader' },
	unknown: { id: 'instance.upgrade.result.unknown', defaultMessage: 'Unavailable' },
	automatic: { id: 'instance.upgrade.result.automatic', defaultMessage: 'Automatic' },
	metrics: { id: 'instance.upgrade.result.metrics', defaultMessage: 'Outcome' },
	updated: { id: 'instance.upgrade.result.updated', defaultMessage: 'Updated' },
	kept: { id: 'instance.upgrade.result.kept', defaultMessage: 'Kept' },
	disabled: { id: 'instance.upgrade.result.disabled', defaultMessage: 'Disabled' },
	added: { id: 'instance.upgrade.result.dependencies-added', defaultMessage: 'Dependencies added' },
	dependencyUpdated: {
		id: 'instance.upgrade.result.dependencies-updated',
		defaultMessage: 'Dependencies updated',
	},
	removed: {
		id: 'instance.upgrade.result.dependencies-removed',
		defaultMessage: 'Dependencies removed',
	},
	openUpgraded: {
		id: 'instance.upgrade.result.open-upgraded',
		defaultMessage: 'Open upgraded instance',
	},
	openOriginal: {
		id: 'instance.upgrade.result.open-original',
		defaultMessage: 'Open original instance',
	},
	backupTitle: { id: 'instance.upgrade.result.backup-title', defaultMessage: 'Backup created' },
	backupDescription: {
		id: 'instance.upgrade.result.backup-description',
		defaultMessage:
			'A complete pre-upgrade copy was created separately from automatic technical rollback.',
	},
	openBackup: { id: 'instance.upgrade.result.open-backup', defaultMessage: 'Open backup' },
	noBackupTitle: {
		id: 'instance.upgrade.result.no-backup-title',
		defaultMessage: 'No complete backup was created',
	},
	noBackupDescription: {
		id: 'instance.upgrade.result.no-backup-description',
		defaultMessage:
			'Automatic technical rollback protected this operation while it was running; it is not a permanent backup.',
	},
	externalTitle: {
		id: 'instance.upgrade.result.external-title',
		defaultMessage: 'Changes detected while upgrading',
	},
	externalDescription: {
		id: 'instance.upgrade.result.external-description',
		defaultMessage:
			'Files changed outside the launcher were detected, and user changes were given priority where applicable.',
	},
	skippedTitle: {
		id: 'instance.upgrade.result.skipped-title',
		defaultMessage: 'Some planned changes were skipped',
	},
	skippedDescription: {
		id: 'instance.upgrade.result.skipped-description',
		defaultMessage: 'These files changed while upgrading, so the user changes were preserved.',
	},
	warningsTitle: {
		id: 'instance.upgrade.result.warnings-title',
		defaultMessage: 'Compatibility warnings',
	},
	warningPrereleaseOnly: {
		id: 'instance.upgrade.warning.prerelease-only',
		defaultMessage: '{path} only has prerelease builds for the target environment.',
	},
	warningUnidentified: {
		id: 'instance.upgrade.warning.unidentified',
		defaultMessage: '{path} could not be identified and was preserved unchanged.',
	},
	warningDependencyConflict: {
		id: 'instance.upgrade.warning.dependency-conflict',
		defaultMessage: '{path} has conflicting dependency requirements.',
	},
	warningMissingDependency: {
		id: 'instance.upgrade.warning.missing-required-dependency',
		defaultMessage: '{path} requires a dependency that could not be resolved.',
	},
	warningIncompatibleDependency: {
		id: 'instance.upgrade.warning.incompatible-dependency',
		defaultMessage: '{path} has an incompatible dependency.',
	},
	warningUnsupportedType: {
		id: 'instance.upgrade.warning.unsupported-content-type',
		defaultMessage: '{path} uses a content type that cannot be upgraded automatically.',
	},
	warningNoRelease: {
		id: 'instance.upgrade.warning.no-compatible-release',
		defaultMessage: '{path} has no compatible release for the target environment.',
	},
	warningNoShaderRuntime: {
		id: 'instance.upgrade.warning.no-compatible-shader-runtime',
		defaultMessage: '{path} has no release compatible with the target shader runtime.',
	},
	warningShaderMissing: {
		id: 'instance.upgrade.warning.shader-runtime-missing',
		defaultMessage: '{path} was preserved because no target shader runtime is configured.',
	},
	warningShaderUnknown: {
		id: 'instance.upgrade.warning.shader-runtime-unknown',
		defaultMessage: '{path} was preserved because the target shader runtime is unknown.',
	},
	warningSearchLimit: {
		id: 'instance.upgrade.warning.search-limit-reached',
		defaultMessage: '{path} could not be resolved within the bounded compatibility search.',
	},
	warningKeepIncompatible: {
		id: 'instance.upgrade.warning.keep-incompatible',
		defaultMessage: '{path} was kept unchanged and may be incompatible with the upgraded instance.',
	},
	detailsTitle: { id: 'instance.upgrade.result.details-title', defaultMessage: 'Upgrade details' },
	add: { id: 'instance.upgrade.result.action-add', defaultMessage: 'Added' },
	upgrade: { id: 'instance.upgrade.result.action-upgrade', defaultMessage: 'Updated' },
	keep: { id: 'instance.upgrade.result.action-keep', defaultMessage: 'Kept' },
	disable: { id: 'instance.upgrade.result.action-disable', defaultMessage: 'Disabled' },
	remove: { id: 'instance.upgrade.result.action-remove', defaultMessage: 'Removed' },
	changeAdded: { id: 'instance.upgrade.result.change-added', defaultMessage: 'Added' },
	changeRemoved: { id: 'instance.upgrade.result.change-removed', defaultMessage: 'Removed' },
	changeModified: { id: 'instance.upgrade.result.change-modified', defaultMessage: 'Modified' },
})

const props = defineProps<{ result: import('@/helpers/instance-upgrade').InstanceUpgradeResult }>()
const router = useRouter()
const { formatMessage } = useVIntl()
const result = computed(() => props.result)
const mode = computed(() => upgradeResultMode(result.value))
const targetEnvironment = computed(() => result.value.targetEnvironment ?? null)
const sourceEnvironment = computed(() => result.value.sourceEnvironment ?? null)
const relatedInstancesQuery = useQuery({
	queryKey: computed(() => [
		'instance-upgrade',
		'result-instances',
		result.value.sourceInstanceId,
		result.value.targetInstanceId,
		result.value.backupInstanceId,
	]),
	queryFn: () =>
		getInstances([
			result.value.sourceInstanceId,
			result.value.targetInstanceId,
			...(result.value.backupInstanceId ? [result.value.backupInstanceId] : []),
		]).catch(() => []),
	staleTime: Number.POSITIVE_INFINITY,
})
const targetInstance = computed(
	() =>
		relatedInstancesQuery.data.value?.find(
			(instance) => instance.id === result.value.targetInstanceId,
		) ?? null,
)
const actualTargetEnvironment = computed<InstanceUpgradeTargetEnvironment | null>(() =>
	targetInstance.value
		? {
				gameVersion: targetInstance.value.game_version,
				modLoader: targetInstance.value.loader,
				modLoaderVersion:
					targetInstance.value.loader_version ?? targetEnvironment.value?.modLoaderVersion ?? null,
				shaderRuntime: targetEnvironment.value?.shaderRuntime ?? 'unknown',
			}
		: targetEnvironment.value,
)
const summary = computed(() => summarizeUpgradeResult(result.value.solution))
const warningRows = computed(() => upgradeResultWarningRows(result.value))
const warningsExpandedByDefault = computed(() =>
	shouldExpandUpgradeWarningsByDefault(warningRows.value.length),
)
const metrics = computed(() => [
	{ label: formatMessage(messages.updated), value: summary.value.updated },
	{ label: formatMessage(messages.kept), value: summary.value.kept },
	{ label: formatMessage(messages.disabled), value: summary.value.disabled },
	{ label: formatMessage(messages.added), value: summary.value.dependencyAdded },
	{ label: formatMessage(messages.dependencyUpdated), value: summary.value.dependencyUpdated },
	{ label: formatMessage(messages.removed), value: summary.value.dependencyRemoved },
])
const releaseIdentities = computed<UpgradeReleaseIdentity[]>(() =>
	result.value.solution.selections
		.flatMap((selection) =>
			[selection.currentReleaseId, selection.targetReleaseId].flatMap((releaseId) =>
				selection.provider && selection.projectId && releaseId
					? [{ provider: selection.provider, projectId: selection.projectId, releaseId }]
					: [],
			),
		)
		.concat(
			result.value.solution.dependencyChanges.flatMap((change) =>
				[change.currentReleaseId, change.targetReleaseId].flatMap((releaseId) =>
					releaseId ? [{ provider: change.provider, projectId: change.projectId, releaseId }] : [],
				),
			),
		),
)
const versionMetadataQuery = useQuery({
	queryKey: computed(() => [
		'instance-upgrade',
		'result-versions',
		...releaseIdentities.value.map(
			(identity) => `${identity.provider}:${identity.projectId}:${identity.releaseId}`,
		),
	]),
	queryFn: () => loadUpgradeVersionDisplayMetadata(releaseIdentities.value),
	staleTime: Number.POSITIVE_INFINITY,
})
const projectIdentities = computed<UpgradeProjectIdentity[]>(() => {
	const identities = new Map<string, UpgradeProjectIdentity>()
	for (const entry of [
		...result.value.solution.selections,
		...result.value.solution.dependencyChanges,
	]) {
		if (entry.provider && entry.projectId) {
			identities.set(`${entry.provider}:${entry.projectId}`, {
				provider: entry.provider,
				projectId: entry.projectId,
			})
		}
	}
	return [...identities.values()]
})
const projectMetadataQuery = useQuery({
	queryKey: computed(() => [
		'instance-upgrade',
		'result-projects',
		...projectIdentities.value.map((identity) => `${identity.provider}:${identity.projectId}`),
	]),
	queryFn: () => loadUpgradeProjectDisplayMetadata(projectIdentities.value),
	staleTime: Number.POSITIVE_INFINITY,
})
const selectionDetails = computed(() =>
	result.value.solution.selections.map((selection) => {
		const key =
			selection.provider && selection.projectId
				? upgradeProjectDisplayCacheKey(selection.provider, selection.projectId)
				: null
		const metadata = key ? projectMetadataQuery.data.value?.get(key) : null
		return {
			key: selection.contentId,
			title: metadata?.title ?? selection.projectId ?? formatMessage(messages.unknown),
			path: upgradeProjectPath(selection.provider, selection.projectId),
			provider: selection.provider,
			projectId: selection.projectId,
			currentReleaseId: selection.currentReleaseId,
			targetReleaseId: selection.targetReleaseId,
			current: selection.currentReleaseId
				? upgradeVersionDisplayLabel(versionMetadataQuery.data.value, {
						provider: selection.provider,
						projectId: selection.projectId,
						releaseId: selection.currentReleaseId,
					})
				: formatMessage(messages.unknown),
			target: selection.targetReleaseId
				? upgradeVersionDisplayLabel(versionMetadataQuery.data.value, {
						provider: selection.provider,
						projectId: selection.projectId,
						releaseId: selection.targetReleaseId,
					})
				: null,
			action: selection.action,
			actionLabel: formatMessage(messages[selection.action]),
		}
	}),
)

function loaderLabel(environment: InstanceUpgradeTargetEnvironment | null) {
	if (!environment) return formatMessage(messages.unknown)
	const label = formatLoaderLabel(environment.modLoader)
	return environment.modLoaderVersion
		? `${label} ${environment.modLoaderVersion}`
		: `${label} (${formatMessage(messages.automatic)})`
}
function projectTitle(provider: string, projectId: string) {
	return (
		projectMetadataQuery.data.value?.get(upgradeProjectDisplayCacheKey(provider, projectId))
			?.title ?? projectId
	)
}
function dependencyKindLabel(kind: string) {
	return formatMessage(
		messages[
			kind === 'add'
				? 'add'
				: kind === 'upgrade'
					? 'upgrade'
					: kind === 'remove'
						? 'remove'
						: 'keep'
		],
	)
}
function externalChangeLabel(kind: InstanceUpgradeExternalChangeKind) {
	return formatMessage(
		messages[
			kind === 'added' ? 'changeAdded' : kind === 'removed' ? 'changeRemoved' : 'changeModified'
		],
	)
}
function warningLabel(warning: UpgradeWarningRow) {
	if (warning.legacyMessage) return warning.legacyMessage
	const descriptor =
		warning.code === 'prerelease_only'
			? messages.warningPrereleaseOnly
			: warning.code === 'unidentified'
				? messages.warningUnidentified
				: warning.code === 'dependency_conflict'
					? messages.warningDependencyConflict
					: warning.code === 'missing_required_dependency'
						? messages.warningMissingDependency
						: warning.code === 'incompatible_dependency'
							? messages.warningIncompatibleDependency
							: warning.code === 'unsupported_content_type'
								? messages.warningUnsupportedType
								: warning.code === 'no_compatible_release'
									? messages.warningNoRelease
									: warning.code === 'no_compatible_shader_runtime'
										? messages.warningNoShaderRuntime
										: warning.code === 'shader_runtime_missing'
											? messages.warningShaderMissing
											: warning.code === 'shader_runtime_unknown'
												? messages.warningShaderUnknown
												: warning.code === 'search_limit_reached'
													? messages.warningSearchLimit
													: messages.warningKeepIncompatible
	return formatMessage(descriptor, {
		path: warning.relativePath ?? formatMessage(messages.unknown),
	})
}
</script>
