import assert from 'node:assert/strict'
import test from 'node:test'

import { clearUpgradeFlow, consumeUpgradeFlow, parkUpgradeFlow, upgradeProjectPath } from '../../../helpers/upgrade-return-state.ts'

test('upgrade return snapshot is one-shot and instance-scoped', () => {
	clearUpgradeFlow()
	const snapshot = {
		instanceId: 'instance-a',
		returnFullPath: '/instance/instance-a/upgrade/compatibility',
		targetEnvironment: null,
		plan: null,
		createFullBackup: true,
		sharedUpgradeMode: null,
		activeJobId: null,
		result: null,
	}
	parkUpgradeFlow(snapshot)
	assert.equal(consumeUpgradeFlow('instance-b', snapshot.returnFullPath), null)
	assert.deepEqual(consumeUpgradeFlow('instance-a', snapshot.returnFullPath), snapshot)
	assert.equal(consumeUpgradeFlow('instance-a', snapshot.returnFullPath), null)
})

test('upgrade project paths only use trusted providers', () => {
	assert.equal(upgradeProjectPath('modrinth', 'P7dR8mSH'), '/project/P7dR8mSH')
	assert.equal(upgradeProjectPath('curseforge', '123'), '/project/curseforge/123')
	assert.equal(upgradeProjectPath('local', 'pack'), null)
})
