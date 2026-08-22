import assert from 'node:assert/strict'
import test from 'node:test'

import type { InstanceUpgradePlan } from '@/helpers/instance-upgrade'

import {
	actionableWarningContentIds,
	availablePredefinedStrategies,
	compatibilitySummary,
	customConstraintsEqual,
	editableUpgradeRoots,
	groupUpgradeIssues,
	contentIdentityKeys,
	inferShaderRuntime,
	newerStableGameVersions,
	setFixedConstraint,
	solutionSummary,
	upgradeContentDisplayMetadata,
} from './analysis.ts'

function issue(code: string, contentId: string | null, projectId: string | null, message = code) {
	return {
		code,
		message,
		contentId,
		provider: projectId ? 'modrinth' : null,
		projectId,
		conflictingProjectId: null,
		dependencyRequirements: [],
	}
}

function planItem(contentId: string, projectId: string | null = contentId) {
	return {
		contentId,
		relativePath: `mods/${contentId}.jar`,
		projectType: 'mod',
		provider: projectId ? 'modrinth' : null,
		projectId,
		currentReleaseId: 'old',
		currentEnabled: true,
		autoDependency: false,
		status: 'already_compatible',
		resolution: {
			contentId,
			action: 'upgrade',
			allowPrerelease: false,
			confirmedPrereleaseDependencies: [],
		},
		candidateReleaseIds: [],
	}
}

test('newer stable versions follow metadata order without numeric parsing', () => {
	const result = newerStableGameVersions(
		[
			{ version: '26.1.2', version_type: 'release', date: '', major: false },
			{ version: '26.1-beta', version_type: 'snapshot', date: '', major: false },
			{ version: '26.1', version_type: 'release', date: '', major: true },
			{ version: '1.21.8', version_type: 'release', date: '', major: false },
			{ version: '1.21.7', version_type: 'release', date: '', major: false },
		],
		'1.21.8',
	)

	assert.deepEqual(result, { currentFound: true, versions: ['26.1.2', '26.1'] })
})

test('unknown current version exposes stable releases conservatively', () => {
	const result = newerStableGameVersions(
		[
			{ version: '26.1', version_type: 'release', date: '', major: true },
			{ version: '26.1-beta', version_type: 'snapshot', date: '', major: false },
		],
		'custom',
	)

	assert.deepEqual(result, { currentFound: false, versions: ['26.1'] })
})

test('compatibility summary uses selected solution and changed dependencies', () => {
	const plan = {
		blockingIssues: [{ code: 'dependency_conflict' }, { code: 'prerelease_only' }],
		selectedSolution: {
			selections: [
				{ action: 'upgrade', currentReleaseId: 'old', targetReleaseId: 'new' },
				{ action: 'keep', currentReleaseId: 'same', targetReleaseId: 'same' },
				{ action: 'disable', currentReleaseId: 'off', targetReleaseId: null },
			],
			dependencyChanges: [{ kind: 'add' }, { kind: 'keep' }, { kind: 'remove' }],
		},
	} as InstanceUpgradePlan

	assert.deepEqual(compatibilitySummary(plan), {
		updates: 1,
		keptOrCompatible: 1,
		disabled: 1,
		dependencyChanges: 2,
		needsAttention: 2,
	})
})

test('shader runtime inference uses exact loader component and provider identity', () => {
	const instance = {
		loader: 'fabric',
		loader_components: [],
	} as never
	const snapshot = {
		items: [
			{
				projectType: 'mod',
				provider: 'modrinth',
				providerProjectId: 'YL57xq9U',
				content: null,
			},
		],
	} as never

	assert.equal(inferShaderRuntime(instance, snapshot), 'iris')
	assert.equal(
		inferShaderRuntime({ ...instance, loader_components: [{ kind: 'optifine' }] }, undefined),
		'opti_fine',
	)
	assert.equal(inferShaderRuntime(instance, undefined), 'unknown')
})

test('solution summary separates root and dependency changes', () => {
	const summary = solutionSummary({
		kind: 'newest',
		selections: [
			{
				contentId: 'a',
				provider: 'modrinth',
				projectId: 'a',
				currentReleaseId: '1',
				targetReleaseId: '2',
				action: 'upgrade',
				enabled: true,
			},
			{
				contentId: 'b',
				provider: 'modrinth',
				projectId: 'b',
				currentReleaseId: '1',
				targetReleaseId: '1',
				action: 'keep',
				enabled: true,
			},
			{
				contentId: 'c',
				provider: 'modrinth',
				projectId: 'c',
				currentReleaseId: '1',
				targetReleaseId: null,
				action: 'disable',
				enabled: false,
			},
		],
		dependencyChanges: [
			{
				existingContentId: null,
				provider: 'modrinth',
				projectId: 'd',
				currentReleaseId: null,
				targetReleaseId: '1',
				kind: 'add',
				enabled: true,
			},
			{
				existingContentId: 'e',
				provider: 'modrinth',
				projectId: 'e',
				currentReleaseId: '1',
				targetReleaseId: '2',
				kind: 'upgrade',
				enabled: true,
			},
			{
				existingContentId: 'f',
				provider: 'modrinth',
				projectId: 'f',
				currentReleaseId: '1',
				targetReleaseId: null,
				kind: 'remove',
				enabled: false,
			},
		],
		warnings: [],
	})
	assert.deepEqual(summary, {
		upgraded: 1,
		kept: 1,
		disabled: 1,
		dependencyAdditions: 1,
		dependencyUpdates: 1,
		dependencyRemovals: 1,
	})
})

test('fixed constraints replace and remove by physical content without duplicates', () => {
	const first = {
		contentId: 'a',
		provider: 'modrinth' as const,
		projectId: 'project',
		versionId: 'one',
	}
	const replaced = setFixedConstraint([first], { ...first, versionId: 'two' }, 'a')
	assert.deepEqual(replaced, [{ ...first, versionId: 'two' }])
	assert.deepEqual(setFixedConstraint(replaced, null, 'a'), [])
	assert.equal(customConstraintsEqual(replaced, [{ ...first, versionId: 'two' }]), true)
})

test('editable roots exclude automatic dependencies', () => {
	const root = {
		contentId: 'root',
		autoDependency: false,
		provider: 'modrinth',
		projectId: 'root',
		candidateReleaseIds: ['one'],
	}
	const dependency = { ...root, contentId: 'dependency', autoDependency: true }
	const plan = { items: [root, dependency], customConstraints: [] } as InstanceUpgradePlan
	assert.deepEqual(
		editableUpgradeRoots(plan).map((item) => item.contentId),
		['root'],
	)
})

test('unavailable minimal solution is not selectable', () => {
	const newestSolution = { kind: 'newest', selections: [], dependencyChanges: [], warnings: [] }
	assert.deepEqual(
		availablePredefinedStrategies({
			newestSolution,
			minimalChangeSolution: null,
		} as InstanceUpgradePlan),
		['newest'],
	)
})

test('issue grouping gives blocking precedence and includes every content once', () => {
	const plan = {
		items: [planItem('blocked'), planItem('warned'), planItem('clear')],
		blockingIssues: [issue('dependency_conflict', 'blocked', 'blocked')],
		warnings: [
			issue('keep_incompatible', 'blocked', 'blocked'),
			issue('keep_incompatible', 'warned', 'warned'),
		],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.deepEqual(
		groups.blocking.map((group) => group.item.contentId),
		['blocked'],
	)
	assert.deepEqual(
		groups.warnings.map((group) => group.item.contentId),
		['warned'],
	)
	assert.deepEqual(
		groups.noIssues.map((group) => group.item.contentId),
		['clear'],
	)
	assert.equal(
		new Set(
			[...groups.blocking, ...groups.warnings, ...groups.noIssues].map(
				(group) => group.item.contentId,
			),
		).size,
		3,
	)
})

test('root and content forms of one issue coalesce on exact project identity', () => {
	const plan = {
		items: [planItem('content', 'project')],
		blockingIssues: [
			issue('no_compatible_release', null, 'project', 'root form'),
			issue('no_compatible_release', 'content', 'project', 'content form'),
		],
		warnings: [],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.equal(groups.blocking[0].blockingIssues.length, 1)
	assert.equal(groups.blocking[0].blockingIssues[0].message, 'content form')
})

test('unmapped and ambiguous project issues remain global', () => {
	const duplicate = planItem('duplicate-b', 'duplicate')
	const plan = {
		items: [planItem('duplicate-a', 'duplicate'), duplicate],
		blockingIssues: [issue('dependency_conflict', null, 'missing')],
		warnings: [issue('keep_incompatible', null, 'duplicate')],
	} as InstanceUpgradePlan
	const groups = groupUpgradeIssues(plan)

	assert.equal(groups.globalBlockingIssues.length, 1)
	assert.equal(groups.globalWarnings.length, 1)
	assert.equal(groups.noIssues.length, 2)
})

test('actionable warning filtering excludes global and informational conflicts', () => {
	const plan = {
		items: [planItem('actionable'), planItem('conflict')],
		blockingIssues: [],
		warnings: [
			issue('keep_incompatible', 'actionable', 'actionable'),
			issue('dependency_conflict', 'conflict', 'conflict'),
			issue('keep_incompatible', null, null),
		],
	} as InstanceUpgradePlan

	assert.deepEqual(actionableWarningContentIds(groupUpgradeIssues(plan)), ['actionable'])
})

test('actionable warning count uses unique content rows', () => {
	const plan = {
		items: [planItem('one'), planItem('two')],
		blockingIssues: [],
		warnings: [
			issue('keep_incompatible', 'one', 'one'),
			issue('shader_runtime_unknown', 'one', 'one'),
			issue('unidentified', 'two', 'two'),
		],
	} as InstanceUpgradePlan

	assert.equal(actionableWarningContentIds(groupUpgradeIssues(plan)).length, 2)
})

test('content display metadata prefers normalized content then snapshot then plan fallback', () => {
	const item = planItem('entry', 'plan-project') as never
	const snapshot = {
		expectedRelativePath: 'resourcepacks/file.zip',
		content: {
			project: { title: 'Snapshot title', icon_url: 'snapshot.png' },
			version: { version_number: 'snapshot-version' },
		},
	} as never
	const content = {
		project: { title: 'Resolved title', icon_url: 'resolved.png' },
		version: { version_number: 'resolved-version' },
	} as never

	assert.deepEqual(upgradeContentDisplayMetadata(item, content, snapshot), {
		title: 'Resolved title',
		iconUrl: 'resolved.png',
		currentVersion: 'resolved-version',
	})
	assert.deepEqual(upgradeContentDisplayMetadata(item, undefined, snapshot), {
		title: 'Snapshot title',
		iconUrl: 'snapshot.png',
		currentVersion: 'snapshot-version',
	})
	assert.equal(upgradeContentDisplayMetadata(item).title, 'entry.jar')
})

test('local content identity joins by normalized path when entry ids are absent', () => {
	assert.deepEqual(
		contentIdentityKeys({ relativePath: 'resourcepacks\\pack.zip' }),
		['resourcepacks/pack.zip'],
	)
	assert.deepEqual(
		contentIdentityKeys({ instanceEntryId: 'entry', relativePath: 'resourcepacks/pack.zip' }),
		['entry', 'resourcepacks/pack.zip'],
	)
})
