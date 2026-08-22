import assert from 'node:assert/strict'
import test from 'node:test'

import type { InstallJobSnapshot, InstallJobStatus } from '@/helpers/install'
import type { InstanceUpgradeResult } from '@/helpers/instance-upgrade'

import {
	isInstanceUpgradeJobWith,
	isRecoverableUpgradeStatus,
	selectRecoverableUpgradeJobWith,
	submitInstanceUpgradeWith,
} from './install-job-core.ts'

function job(
	jobId: string,
	status: InstallJobStatus,
	options: {
		instanceId?: string
		kind?: InstallJobSnapshot['kind']
		modified?: string
		executionMode?: InstallJobSnapshot['execution_mode']
		result?: InstanceUpgradeResult | null
	} = {},
): InstallJobSnapshot {
	return {
		job_id: jobId,
		instance_id: options.instanceId ?? 'instance-a',
		kind: options.kind ?? 'upgrade_unmanaged_instance',
		status,
		execution_mode: options.executionMode ?? 'normal',
		target: { type: 'existing_instance', instance_id: options.instanceId ?? 'instance-a' },
		modified: options.modified ?? '2026-08-22T10:00:00Z',
		created: '2026-08-22T09:00:00Z',
		upgrade_result: options.result,
	} as InstallJobSnapshot
}

const result = { planId: 'plan-a' } as InstanceUpgradeResult
const instanceIdOf = (candidate: InstallJobSnapshot) =>
	candidate.instance_id ?? candidate.target.instance_id ?? null

test('upgrade job ownership requires matching kind and instance identity', () => {
	assert.equal(
		isInstanceUpgradeJobWith(job('correct', 'running'), 'instance-a', instanceIdOf),
		true,
	)
	assert.equal(
		isInstanceUpgradeJobWith(
			job('wrong-kind', 'running', { kind: 'install_content' }),
			'instance-a',
			instanceIdOf,
		),
		false,
	)
	assert.equal(
		isInstanceUpgradeJobWith(
			job('wrong-instance', 'running', { instanceId: 'instance-b' }),
			'instance-a',
			instanceIdOf,
		),
		false,
	)
})

test('active recovery includes waiting and recovery validation and chooses freshest job', () => {
	assert.equal(isRecoverableUpgradeStatus('waiting_for_user'), true)
	const selected = selectRecoverableUpgradeJobWith(
		[
			job('older', 'running', { modified: '2026-08-22T10:00:00Z' }),
			job('newer', 'waiting_for_user', {
				modified: '2026-08-22T11:00:00Z',
				executionMode: 'recovery_validation',
			}),
		],
		'instance-a',
		{},
		instanceIdOf,
	)
	assert.equal(selected?.job_id, 'newer')
})

test('ordinary entry ignores old success while continuation recovers backend result', () => {
	const succeeded = job('succeeded', 'succeeded', { result })
	assert.equal(selectRecoverableUpgradeJobWith([succeeded], 'instance-a', {}, instanceIdOf), null)
	assert.equal(
		selectRecoverableUpgradeJobWith([succeeded], 'instance-a', { continuation: true }, instanceIdOf)
			?.upgrade_result,
		result,
	)
})

test('known terminal job preserves flow ownership', () => {
	const failed = job('known', 'failed')
	assert.equal(
		selectRecoverableUpgradeJobWith([failed], 'instance-a', { knownJobId: 'known' }, instanceIdOf)
			?.job_id,
		'known',
	)
})

function submissionDependencies(calls: unknown[][], jobs: InstallJobSnapshot[] = []) {
	return {
		instanceIdOf,
		listJobs: async () => jobs,
		execute: async (planId: string, backup: boolean, mode: 'direct' | 'copy_and_upgrade') => {
			calls.push([planId, backup, mode])
			return job(`job-${calls.length}`, 'queued')
		},
	}
}

test('normal, shared direct, and copy submissions pass exact execution parameters', async () => {
	const calls: unknown[][] = []
	const dependencies = submissionDependencies(calls)
	for (const request of [
		{
			instanceId: 'instance-a',
			planId: 'normal',
			createFullBackup: true,
			sharedUpgradeMode: 'direct' as const,
		},
		{
			instanceId: 'instance-a',
			planId: 'shared-direct',
			createFullBackup: false,
			sharedUpgradeMode: 'direct' as const,
		},
		{
			instanceId: 'instance-a',
			planId: 'copy',
			createFullBackup: false,
			sharedUpgradeMode: 'copy_and_upgrade' as const,
		},
	]) {
		await submitInstanceUpgradeWith(request, { value: false }, dependencies)
	}
	assert.deepEqual(calls, [
		['normal', true, 'direct'],
		['shared-direct', false, 'direct'],
		['copy', false, 'copy_and_upgrade'],
	])
})

test('synchronous lock prevents double submission', async () => {
	let releaseList: (() => void) | undefined
	let executeCalls = 0
	const lock = { value: false }
	const dependencies = {
		instanceIdOf,
		listJobs: () =>
			new Promise<InstallJobSnapshot[]>((resolve) => {
				releaseList = () => resolve([])
			}),
		execute: async () => {
			executeCalls += 1
			return job('started', 'queued')
		},
	}
	const request = {
		instanceId: 'instance-a',
		planId: 'plan-a',
		createFullBackup: true,
		sharedUpgradeMode: 'direct' as const,
	}
	const first = submitInstanceUpgradeWith(request, lock, dependencies)
	const second = submitInstanceUpgradeWith(request, lock, dependencies)
	assert.equal(await second, null)
	releaseList?.()
	await first
	assert.equal(executeCalls, 1)
})

test('active preflight attaches without a second execution', async () => {
	const calls: unknown[][] = []
	const submitted = await submitInstanceUpgradeWith(
		{
			instanceId: 'instance-a',
			planId: 'plan-a',
			createFullBackup: true,
			sharedUpgradeMode: 'direct',
		},
		{ value: false },
		submissionDependencies(calls, [job('existing', 'running')]),
	)
	assert.equal(submitted?.attached, true)
	assert.equal(submitted?.job.job_id, 'existing')
	assert.equal(calls.length, 0)
})

test('submission failure releases lock', async () => {
	const lock = { value: false }
	await assert.rejects(
		submitInstanceUpgradeWith(
			{
				instanceId: 'instance-a',
				planId: 'plan-a',
				createFullBackup: true,
				sharedUpgradeMode: 'direct',
			},
			lock,
			{
				instanceIdOf,
				listJobs: async () => [],
				execute: async () => {
					throw new Error('stale plan')
				},
			},
		),
		/stale plan/,
	)
	assert.equal(lock.value, false)
})
