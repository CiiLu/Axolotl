import assert from 'node:assert/strict'
import test from 'node:test'

import {
	buildGcCandidateChain,
	detectGcStrategy,
	GC_STRATEGY_DEFINITIONS,
} from './strategies.ts'

function createContext(overrides: Partial<Parameters<typeof buildGcCandidateChain>[0]> = {}) {
	return {
		javaMajorVersion: 21,
		allocatedMemoryMb: 16384,
		systemCpuCores: 16,
		systemLogicalProcessors: 16,
		modCount: 100,
		loader: 'forge',
		...overrides,
	}
}

test('Mojang G1GC args include required parameters', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	assert.ok(args.includes('-XX:+UseG1GC'))
	assert.ok(args.includes('-XX:G1UncommitBias=1'))
	assert.ok(args.includes('-XX:G1HeapRegionSize=32M'))
})

test('PCL G1GC args include required parameters', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-pcl'].buildArgs()
	assert.ok(args.includes('-XX:+UseG1GC'))
	assert.ok(args.includes('-XX:MaxGCPauseMillis=50'))
	assert.ok(!args.includes('-XX:G1UncommitBias=1'))
})

test('Shenandoah args include required parameters', () => {
	const args = GC_STRATEGY_DEFINITIONS.shenandoah.buildArgs()
	assert.ok(args.includes('-XX:+UseShenandoahGC'))
	assert.ok(args.includes('-XX:ShenandoahHeapRegionSize=256M'))
})

test('ZGC args include -XX:+ZGenerational for Java 21+', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs({
		javaMajorVersion: 21,
		allocatedMemoryMb: 8192,
		systemCpuCores: 8,
		systemLogicalProcessors: 8,
		modCount: 0,
		loader: 'fabric',
	})
	assert.ok(args.includes('-XX:+UseZGC'))
	assert.ok(args.includes('-XX:+ZGenerational'))
})

test('ZGC args do not include -XX:+ZGenerational for Java < 21', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs({
		javaMajorVersion: 17,
		allocatedMemoryMb: 8192,
		systemCpuCores: 8,
		systemLogicalProcessors: 8,
		modCount: 0,
		loader: 'fabric',
	})
	assert.ok(args.includes('-XX:+UseZGC'))
	assert.ok(!args.includes('-XX:+ZGenerational'))
})

test('detectGcStrategy correctly identifies Mojang G1GC', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	assert.equal(detectGcStrategy(args), 'g1gc-mojang')
})

test('detectGcStrategy correctly identifies PCL G1GC', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-pcl'].buildArgs()
	assert.equal(detectGcStrategy(args), 'g1gc-pcl')
})

test('detectGcStrategy correctly identifies Shenandoah', () => {
	const args = GC_STRATEGY_DEFINITIONS.shenandoah.buildArgs()
	assert.equal(detectGcStrategy(args), 'shenandoah')
})

test('detectGcStrategy correctly identifies ZGC', () => {
	const args = GC_STRATEGY_DEFINITIONS.zgc.buildArgs()
	assert.equal(detectGcStrategy(args), 'zgc')
})

test('detectGcStrategy returns null for unknown strategy', () => {
	assert.equal(detectGcStrategy('-Xmx4G'), null)
})

test('Mojang G1GC is not misidentified as PCL', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs()
	const detected = detectGcStrategy(args)
	assert.equal(detected, 'g1gc-mojang')
	assert.notEqual(detected, 'g1gc-pcl')
})

test('PCL G1GC is not misidentified as Mojang', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-pcl'].buildArgs()
	const detected = detectGcStrategy(args)
	assert.equal(detected, 'g1gc-pcl')
	assert.notEqual(detected, 'g1gc-mojang')
})

test('Mojang G1GC omits -XX:G1UncommitBias before Java 21', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs(createContext({ javaMajorVersion: 17 }))
	assert.ok(args.includes('-XX:+UseG1GC'))
	assert.ok(!args.includes('-XX:G1UncommitBias=1'))
})

test('Mojang G1GC includes -XX:G1UncommitBias on Java 21+', () => {
	const args = GC_STRATEGY_DEFINITIONS['g1gc-mojang'].buildArgs(createContext({ javaMajorVersion: 21 }))
	assert.ok(args.includes('-XX:G1UncommitBias=1'))
})

test('buildGcCandidateChain puts preferred first, dedupes, ends at minimal G1', () => {
	const { ids, args } = buildGcCandidateChain(createContext(), 'zgc')
	assert.deepEqual(ids, ['zgc', 'shenandoah', 'g1gc-mojang', 'g1gc-pcl', 'minimal-g1'])
	assert.equal(args.length, ids.length)
	assert.deepEqual(args[args.length - 1], ['-XX:+UseG1GC'])
})

test('buildGcCandidateChain starts at a non-ZGC preferred strategy', () => {
	const { ids } = buildGcCandidateChain(createContext(), 'shenandoah')
	assert.deepEqual(ids, ['shenandoah', 'g1gc-mojang', 'g1gc-pcl', 'minimal-g1'])
})

test('buildGcCandidateChain never repeats the preferred strategy', () => {
	for (const preferred of ['zgc', 'shenandoah', 'g1gc-mojang', 'g1gc-pcl']) {
		const { ids } = buildGcCandidateChain(createContext(), preferred)
		assert.equal(ids[0], preferred)
		assert.equal(new Set(ids).size, ids.length)
	}
})
