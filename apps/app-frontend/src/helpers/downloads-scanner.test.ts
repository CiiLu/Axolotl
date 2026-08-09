import assert from 'node:assert/strict'
import test from 'node:test'

import {
	createDownloadsScanLoop,
	createDownloadsScannerPresentationState,
	reduceDownloadsScannerPresentation,
} from './downloads-scanner.ts'

function deferred<T>() {
	let resolve!: (value: T) => void
	const promise = new Promise<T>((complete) => {
		resolve = complete
	})
	return { promise, resolve }
}

test('close and reopen ignores stale scan results', async () => {
	const first = deferred<string>()
	const second = deferred<string>()
	const results: string[] = []
	let scanCount = 0
	const loop = createDownloadsScanLoop({
		scan: () => (scanCount++ === 0 ? first.promise : second.promise),
		onResult: (result) => results.push(result),
		schedule: () => undefined,
		cancelSchedule: () => undefined,
	})

	loop.start()
	const staleRun = loop.runNow()
	loop.stop()
	loop.start()
	first.resolve('stale')
	await staleRun
	assert.deepEqual(results, [])

	const currentRun = loop.runNow()
	second.resolve('current')
	await currentRun
	assert.deepEqual(results, ['current'])
	loop.stop()
})

test('one scan stays in flight at a time', async () => {
	const pending = deferred<string>()
	let calls = 0
	const loop = createDownloadsScanLoop({
		scan: () => {
			calls += 1
			return pending.promise
		},
		onResult: () => undefined,
		schedule: () => undefined,
		cancelSchedule: () => undefined,
	})

	loop.start()
	const firstRun = loop.runNow()
	await loop.runNow()
	assert.equal(calls, 1)
	pending.resolve('done')
	await firstRun
	loop.stop()
})

test('scan start does not override candidate verification presentation', () => {
	let state = createDownloadsScannerPresentationState()
	state = reduceDownloadsScannerPresentation(state, {
		type: 'items_updated',
		items: [{ id: 'mods/example.jar', status: 'verifying' }],
	})
	assert.equal(state.phase, 'verifying')

	state = reduceDownloadsScannerPresentation(state, { type: 'scan_started' })
	assert.equal(state.phase, 'verifying')
	assert.deepEqual(state.verifyingItemIds, ['mods/example.jar'])
})

test('same-name rejection remains visible across scan ticks', () => {
	let state = createDownloadsScannerPresentationState()
	state = reduceDownloadsScannerPresentation(state, {
		type: 'scan_result',
		downloadDirectory: 'C:\\Downloads',
		importedItemIds: [],
		mismatchedItemIds: ['mods/example.jar'],
		pendingCandidates: 0,
		hasErrors: false,
		items: [],
	})
	assert.equal(state.phase, 'rejected')

	state = reduceDownloadsScannerPresentation(state, { type: 'scan_started' })
	assert.equal(state.phase, 'rejected')
	state = reduceDownloadsScannerPresentation(state, { type: 'scan_finished' })
	assert.equal(state.phase, 'rejected')
	assert.deepEqual(state.rejectedItemIds, ['mods/example.jar'])
})

test('changed candidate clears rejection and can return to verification', () => {
	let state = createDownloadsScannerPresentationState()
	state = reduceDownloadsScannerPresentation(state, {
		type: 'scan_result',
		downloadDirectory: 'C:\\Downloads',
		importedItemIds: [],
		mismatchedItemIds: ['mods/example.jar'],
		pendingCandidates: 0,
		hasErrors: false,
		items: [],
	})

	state = reduceDownloadsScannerPresentation(state, {
		type: 'scan_result',
		downloadDirectory: 'C:\\Downloads',
		importedItemIds: [],
		mismatchedItemIds: [],
		pendingCandidates: 1,
		hasErrors: false,
		items: [],
	})
	assert.equal(state.phase, 'waiting_for_stability')
	assert.deepEqual(state.rejectedItemIds, [])

	state = reduceDownloadsScannerPresentation(state, {
		type: 'items_updated',
		items: [{ id: 'mods/example.jar', status: 'verifying' }],
	})
	assert.equal(state.phase, 'verifying')
	assert.deepEqual(state.verifyingItemIds, ['mods/example.jar'])
})

test('successful import clears candidate rejection and verification', () => {
	let state = createDownloadsScannerPresentationState()
	state = reduceDownloadsScannerPresentation(state, {
		type: 'scan_result',
		downloadDirectory: 'C:\\Downloads',
		importedItemIds: [],
		mismatchedItemIds: ['mods/example.jar'],
		pendingCandidates: 0,
		hasErrors: false,
		items: [],
	})
	state = reduceDownloadsScannerPresentation(state, {
		type: 'items_updated',
		items: [{ id: 'mods/example.jar', status: 'verifying' }],
	})

	state = reduceDownloadsScannerPresentation(state, {
		type: 'scan_result',
		downloadDirectory: 'C:\\Downloads',
		importedItemIds: ['mods/example.jar'],
		mismatchedItemIds: [],
		pendingCandidates: 0,
		hasErrors: false,
		items: [],
	})
	assert.equal(state.phase, 'imported')
	assert.deepEqual(state.rejectedItemIds, [])
	assert.deepEqual(state.verifyingItemIds, [])
	assert.equal(state.importedCount, 1)
})
