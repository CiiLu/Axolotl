import type { GcStrategyDefinition, GcStrategyId, ResolvedGcStrategyId } from './types'

function buildG1gcMojangArgs(): string {
	return [
		'-XX:+UseG1GC',
		'-XX:G1UncommitBias=1',
		'-XX:G1HeapRegionSize=32M',
		'-XX:+UnlockExperimentalVMOptions',
		'-XX:G1NewSizePercent=20',
		'-XX:G1ReservePercent=20',
		'-XX:MaxGCPauseMillis=0',
		'-XX:G1HeapWastePercent=0',
		'-XX:G1MixedGCCountTarget=4',
		'-XX:InitiatingHeapOccupancyPercent=10',
	].join(' ')
}

function buildG1gcPclArgs(): string {
	return [
		'-XX:+UseG1GC',
		'-XX:MaxGCPauseMillis=50',
		'-XX:G1HeapRegionSize=32M',
		'-XX:G1NewSizePercent=20',
		'-XX:G1ReservePercent=20',
		'-XX:InitiatingHeapOccupancyPercent=15',
		'-XX:G1MixedGCCountTarget=4',
	].join(' ')
}

function buildShenandoahArgs(): string {
	return [
		'-XX:+UseShenandoahGC',
		'-XX:ShenandoahHeapRegionSize=256M',
		'-XX:+UnlockExperimentalVMOptions',
	].join(' ')
}

function buildZgcArgs(javaMajorVersion: number | null): string {
	const args = ['-XX:+UseZGC', '-XX:+UnlockExperimentalVMOptions']
	if (javaMajorVersion !== null && javaMajorVersion >= 21) {
		args.push('-XX:+ZGenerational')
	}
	return args.join(' ')
}

function detectG1gcMojang(args: string): boolean {
	return args.includes('-XX:+UseG1GC') && args.includes('-XX:G1UncommitBias=1')
}

function detectG1gcPcl(args: string): boolean {
	return (
		args.includes('-XX:+UseG1GC') &&
		args.includes('-XX:G1NewSizePercent=20') &&
		!args.includes('-XX:G1UncommitBias=1')
	)
}

function detectShenandoah(args: string): boolean {
	return args.includes('-XX:+UseShenandoahGC')
}

function detectZgc(args: string): boolean {
	return args.includes('-XX:+UseZGC')
}

export const GC_STRATEGY_DEFINITIONS: Record<ResolvedGcStrategyId, GcStrategyDefinition> = {
	'g1gc-mojang': {
		id: 'g1gc-mojang',
		baseArgs: buildG1gcMojangArgs(),
		detect: detectG1gcMojang,
		buildArgs: () => buildG1gcMojangArgs(),
	},
	'g1gc-pcl': {
		id: 'g1gc-pcl',
		baseArgs: buildG1gcPclArgs(),
		detect: detectG1gcPcl,
		buildArgs: () => buildG1gcPclArgs(),
	},
	shenandoah: {
		id: 'shenandoah',
		baseArgs: buildShenandoahArgs(),
		detect: detectShenandoah,
		buildArgs: () => buildShenandoahArgs(),
	},
	zgc: {
		id: 'zgc',
		baseArgs: buildZgcArgs(null),
		detect: detectZgc,
		buildArgs: (context) => buildZgcArgs(context?.javaMajorVersion ?? null),
	},
}

export function detectGcStrategy(args: string): ResolvedGcStrategyId | null {
	for (const [strategyId, definition] of Object.entries(GC_STRATEGY_DEFINITIONS)) {
		if (definition.detect(args)) {
			return strategyId as ResolvedGcStrategyId
		}
	}
	return null
}

export function getStrategyBaseArgs(strategyId: GcStrategyId): string {
	if (strategyId === 'auto') {
		return GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs
	}
	return GC_STRATEGY_DEFINITIONS[strategyId].baseArgs
}
