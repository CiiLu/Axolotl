import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import type { InstanceUpgradeResult } from '@/helpers/instance-upgrade'

import { upgradeResultWarningRows, upgradeWarningMessageId } from './upgrade-warning.ts'

const base = {
	planId: 'plan',
	sourceInstanceId: 'source',
	targetInstanceId: 'target',
	backupInstanceId: null,
	solution: { kind: 'custom', selections: [], dependencyChanges: [], warnings: [] },
	externalChanges: [],
	skippedDueToExternalConflict: [],
} as InstanceUpgradeResult

test('structured warning maps by stable code', () => {
	const rows = upgradeResultWarningRows({
		...base,
		compatibilityWarnings: [],
		compatibilityWarningDetails: [
			{
				code: 'keep_incompatible',
				relativePath: 'mods/a.jar',
				contentId: 'a',
				provider: 'modrinth',
				projectId: 'project',
				conflictingProjectId: null,
			},
		],
	})
	assert.equal(upgradeWarningMessageId(rows[0].code!), 'instance.upgrade.warning.keep-incompatible')
	const zhCn = JSON.parse(
		readFileSync(new URL('../../../locales/zh-CN/index.json', import.meta.url), 'utf8'),
	) as Record<string, { message: string }>
	const localized = zhCn[upgradeWarningMessageId(rows[0].code!)]?.message
	assert.equal(localized, '{path} 已原样保留，可能与升级后的实例不兼容。')
	assert.doesNotMatch(localized, /will be preserved/i)
})

test('legacy persisted warning falls back to raw message', () => {
	const rows = upgradeResultWarningRows({
		...base,
		compatibilityWarnings: [
			{
				code: 'unidentified',
				message: 'Legacy backend text',
				contentId: null,
				provider: null,
				projectId: null,
				conflictingProjectId: null,
				dependencyRequirements: [],
			},
		],
	})
	assert.equal(rows[0].legacyMessage, 'Legacy backend text')
})
