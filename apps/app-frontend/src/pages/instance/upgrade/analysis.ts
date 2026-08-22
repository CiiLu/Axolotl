import type { ContentItem } from '@modrinth/ui'
import type { GameVersionTag } from '@modrinth/utils'

import type { InstanceContentSnapshot, InstanceContentSnapshotItem } from '@/helpers/instance'
import type {
	InstanceUpgradeFixedConstraint,
	InstanceUpgradeIssue,
	InstanceUpgradePlan,
	InstanceUpgradePlanItem,
	InstanceUpgradeSolution,
	ShaderRuntime,
} from '@/helpers/instance-upgrade'
import type { GameInstance } from '@/helpers/types'

export interface UpgradeVersionTargets {
	currentFound: boolean
	versions: string[]
}

export interface CompatibilitySummary {
	updates: number
	keptOrCompatible: number
	disabled: number
	dependencyChanges: number
	needsAttention: number
}

export interface SolutionSummary {
	upgraded: number
	kept: number
	disabled: number
	dependencyAdditions: number
	dependencyUpdates: number
	dependencyRemovals: number
}

export interface UpgradeContentIssueGroup {
	item: InstanceUpgradePlanItem
	blockingIssues: InstanceUpgradeIssue[]
	warnings: InstanceUpgradeIssue[]
}

export interface UpgradeIssueGroups {
	blocking: UpgradeContentIssueGroup[]
	warnings: UpgradeContentIssueGroup[]
	noIssues: UpgradeContentIssueGroup[]
	globalBlockingIssues: InstanceUpgradeIssue[]
	globalWarnings: InstanceUpgradeIssue[]
}

export interface UpgradeContentDisplayMetadata {
	title: string
	iconUrl: string | null
	currentVersion: string | null
}

export function normalizeUpgradePath(path: string): string {
	return path.replaceAll('\\', '/').replace(/\/+/g, '/').replace(/^\.\//, '')
}

export function contentIdentityKeys(item: {
	contentId?: string | null
	relativePath?: string | null
	instanceEntryId?: string | null
	instanceMemberId?: string | null
	instanceFileId?: string | null
	id?: string | null
	file_path?: string | null
}): string[] {
	return [
		item.contentId,
		item.instanceEntryId,
		item.instanceMemberId,
		item.instanceFileId,
		item.id,
		item.relativePath ? normalizeUpgradePath(item.relativePath) : null,
		item.file_path ? normalizeUpgradePath(item.file_path) : null,
	].filter((value): value is string => Boolean(value))
}

const ACTIONABLE_WARNING_CODES = new Set<InstanceUpgradeIssue['code']>([
	'unidentified',
	'unsupported_content_type',
	'prerelease_only',
	'no_compatible_release',
	'no_compatible_shader_runtime',
	'shader_runtime_missing',
	'shader_runtime_unknown',
	'keep_incompatible',
])

function issueIdentity(issue: InstanceUpgradeIssue): string {
	const requirements = issue.dependencyRequirements
		.map((requirement) =>
			[
				requirement.rootContentId,
				requirement.parentProvider,
				requirement.parentProjectId,
				requirement.parentReleaseId,
				requirement.dependencyProvider,
				requirement.dependencyProjectId,
				requirement.requiredReleaseId ?? '',
				requirement.candidateReleaseId ?? '',
			].join(':'),
		)
		.sort()
		.join('|')
	return [
		issue.code,
		issue.provider ?? '',
		issue.projectId ?? '',
		issue.conflictingProjectId ?? '',
		requirements,
	].join(':')
}

function issueContentId(
	issue: InstanceUpgradeIssue,
	itemsById: Map<string, InstanceUpgradePlanItem>,
	itemsByProject: Map<string, InstanceUpgradePlanItem | null>,
): string | null {
	if (issue.contentId && itemsById.has(issue.contentId)) return issue.contentId
	if (!issue.projectId) return null
	const providerProject = `${issue.provider ?? ''}:${issue.projectId}`
	return itemsByProject.get(providerProject)?.contentId ?? null
}

export function groupUpgradeIssues(plan: InstanceUpgradePlan): UpgradeIssueGroups {
	const itemsById = new Map(plan.items.map((item) => [item.contentId, item]))
	const itemsByProject = new Map<string, InstanceUpgradePlanItem | null>()
	for (const item of plan.items) {
		if (!item.projectId) continue
		const key = `${item.provider ?? ''}:${item.projectId}`
		itemsByProject.set(key, itemsByProject.has(key) ? null : item)
	}

	const blockingByContent = new Map<string, Map<string, InstanceUpgradeIssue>>()
	const warningByContent = new Map<string, Map<string, InstanceUpgradeIssue>>()
	const globalBlockingIssues: InstanceUpgradeIssue[] = []
	const globalWarnings: InstanceUpgradeIssue[] = []

	function collect(
		issue: InstanceUpgradeIssue,
		byContent: Map<string, Map<string, InstanceUpgradeIssue>>,
		global: InstanceUpgradeIssue[],
	) {
		const contentId = issueContentId(issue, itemsById, itemsByProject)
		if (!contentId) {
			global.push(issue)
			return
		}
		const issues = byContent.get(contentId) ?? new Map<string, InstanceUpgradeIssue>()
		const key = issueIdentity(issue)
		const existing = issues.get(key)
		if (!existing || (existing.contentId === null && issue.contentId !== null))
			issues.set(key, issue)
		byContent.set(contentId, issues)
	}

	for (const issue of plan.blockingIssues) collect(issue, blockingByContent, globalBlockingIssues)
	for (const issue of plan.warnings) collect(issue, warningByContent, globalWarnings)

	const blocking: UpgradeContentIssueGroup[] = []
	const warnings: UpgradeContentIssueGroup[] = []
	const noIssues: UpgradeContentIssueGroup[] = []
	for (const item of plan.items) {
		const itemBlocking = [...(blockingByContent.get(item.contentId)?.values() ?? [])]
		const blockingKeys = new Set(itemBlocking.map(issueIdentity))
		const itemWarnings = [...(warningByContent.get(item.contentId)?.values() ?? [])].filter(
			(issue) => !blockingKeys.has(issueIdentity(issue)),
		)
		const group = { item, blockingIssues: itemBlocking, warnings: itemWarnings }
		if (itemBlocking.length) blocking.push(group)
		else if (itemWarnings.length) warnings.push(group)
		else noIssues.push(group)
	}

	return { blocking, warnings, noIssues, globalBlockingIssues, globalWarnings }
}

export function actionableWarningContentIds(groups: UpgradeIssueGroups): string[] {
	return groups.warnings
		.filter((group) => group.warnings.some((issue) => ACTIONABLE_WARNING_CODES.has(issue.code)))
		.map((group) => group.item.contentId)
}

export function upgradeContentDisplayMetadata(
	item: InstanceUpgradePlanItem,
	contentItem?: ContentItem,
	snapshotItem?: InstanceContentSnapshotItem,
): UpgradeContentDisplayMetadata {
	const fallbackPath = snapshotItem?.expectedRelativePath ?? item.relativePath
	const fallbackName = fallbackPath.split('/').pop() ?? fallbackPath
	return {
		title: contentItem?.project.title ?? snapshotItem?.content?.project.title ?? fallbackName,
		iconUrl: contentItem?.project.icon_url ?? snapshotItem?.content?.project.icon_url ?? null,
		currentVersion:
			contentItem?.version?.version_number ??
			snapshotItem?.content?.version?.version_number ??
			item.currentReleaseId,
	}
}

export function availablePredefinedStrategies(plan: InstanceUpgradePlan) {
	return [
		...(plan.newestSolution ? (['newest'] as const) : []),
		...(plan.minimalChangeSolution ? (['minimal_change'] as const) : []),
	]
}

const IRIS_MODRINTH_PROJECT_ID = 'YL57xq9U'

export function inferShaderRuntime(
	instance: GameInstance,
	snapshot: InstanceContentSnapshot | undefined,
): ShaderRuntime {
	if (
		instance.loader === 'optifine' ||
		instance.loader_components.some((component) => component.kind === 'optifine')
	) {
		return 'opti_fine'
	}
	if (!snapshot) return 'unknown'

	const hasIris = snapshot.items.some(
		(item) =>
			(item.provider === 'modrinth' && item.providerProjectId === IRIS_MODRINTH_PROJECT_ID) ||
			item.content?.provider_refs.some(
				(reference) =>
					reference.provider === 'modrinth' && reference.project_id === IRIS_MODRINTH_PROJECT_ID,
			),
	)
	if (hasIris) return 'iris'

	const hasUnresolvedModIdentity = snapshot.items.some(
		(item) =>
			item.projectType === 'mod' &&
			(item.provider !== 'modrinth' || item.providerProjectId === null),
	)
	return hasUnresolvedModIdentity ? 'unknown' : 'none'
}

export function newerStableGameVersions(
	metadata: GameVersionTag[],
	currentVersion: string,
): UpgradeVersionTargets {
	const currentIndex = metadata.findIndex((version) => version.version === currentVersion)
	const candidates = currentIndex === -1 ? metadata : metadata.slice(0, currentIndex)
	return {
		currentFound: currentIndex !== -1,
		versions: candidates
			.filter((version) => version.version_type === 'release' && version.version !== currentVersion)
			.map((version) => version.version),
	}
}

function summarizeSelections(solution: InstanceUpgradeSolution) {
	return solution.selections.reduce(
		(summary, selection) => {
			if (selection.action === 'disable') summary.disabled += 1
			else if (
				selection.action === 'upgrade' &&
				selection.targetReleaseId !== null &&
				selection.targetReleaseId !== selection.currentReleaseId
			) {
				summary.updates += 1
			} else summary.keptOrCompatible += 1
			return summary
		},
		{ updates: 0, keptOrCompatible: 0, disabled: 0 },
	)
}

export function solutionSummary(solution: InstanceUpgradeSolution): SolutionSummary {
	const selections = summarizeSelections(solution)
	return {
		upgraded: selections.updates,
		kept: selections.keptOrCompatible,
		disabled: selections.disabled,
		dependencyAdditions: solution.dependencyChanges.filter((change) => change.kind === 'add')
			.length,
		dependencyUpdates: solution.dependencyChanges.filter((change) => change.kind === 'upgrade')
			.length,
		dependencyRemovals: solution.dependencyChanges.filter((change) => change.kind === 'remove')
			.length,
	}
}

function normalizedConstraints(constraints: InstanceUpgradeFixedConstraint[]) {
	return constraints
		.map((constraint) => ({
			contentId: constraint.contentId,
			provider: constraint.provider,
			projectId: constraint.projectId,
			versionId: constraint.versionId,
		}))
		.sort((left, right) => left.contentId.localeCompare(right.contentId))
}

export function customConstraintsEqual(
	left: InstanceUpgradeFixedConstraint[],
	right: InstanceUpgradeFixedConstraint[],
): boolean {
	return (
		JSON.stringify(normalizedConstraints(left)) === JSON.stringify(normalizedConstraints(right))
	)
}

export function setFixedConstraint(
	constraints: InstanceUpgradeFixedConstraint[],
	constraint: InstanceUpgradeFixedConstraint | null,
	contentId: string,
): InstanceUpgradeFixedConstraint[] {
	const withoutContent = constraints.filter((current) => current.contentId !== contentId)
	return normalizedConstraints(constraint ? [...withoutContent, constraint] : withoutContent)
}

export function editableUpgradeRoots(plan: InstanceUpgradePlan) {
	return plan.items.filter(
		(item) =>
			!item.autoDependency &&
			(item.provider === 'modrinth' || item.provider === 'curseforge') &&
			item.projectId !== null &&
			(item.candidateReleaseIds.length > 0 ||
				plan.customConstraints.some((constraint) => constraint.contentId === item.contentId)),
	)
}

export function compatibilitySummary(plan: InstanceUpgradePlan): CompatibilitySummary {
	const content = plan.selectedSolution
		? summarizeSelections(plan.selectedSolution)
		: plan.items.reduce(
				(summary, item) => {
					if (item.resolution.action === 'disable') summary.disabled += 1
					else if (item.status === 'upgrade_available') summary.updates += 1
					else if (item.status === 'already_compatible' || item.resolution.action === 'keep') {
						summary.keptOrCompatible += 1
					}
					return summary
				},
				{ updates: 0, keptOrCompatible: 0, disabled: 0 },
			)
	const dependencyChanges = (
		plan.selectedSolution?.dependencyChanges ?? plan.dependencyChanges
	).filter((change) => change.kind !== 'keep').length

	return {
		...content,
		dependencyChanges,
		needsAttention: plan.blockingIssues.length,
	}
}
