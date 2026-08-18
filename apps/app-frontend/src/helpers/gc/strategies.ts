import type { GcContext, GcStrategyDefinition, GcStrategyId, ResolvedGcStrategyId } from './types'

function buildG1gcMojangArgs(javaMajorVersion: number | null): string {
	const args = [
		'-XX:+UseG1GC',
		'-XX:G1HeapRegionSize=32M',
		'-XX:+UnlockExperimentalVMOptions',
		'-XX:G1NewSizePercent=20',
		'-XX:G1ReservePercent=20',
		'-XX:MaxGCPauseMillis=0',
		'-XX:G1HeapWastePercent=0',
		'-XX:G1MixedGCCountTarget=4',
		'-XX:InitiatingHeapOccupancyPercent=10',
	]
	// `-XX:G1UncommitBias` is only understood by newer JDKs (21+). Don't emit
	// it for older runtimes where the JVM would refuse to start — the launch
	// routine verifies and can prune it regardless.
	if (javaMajorVersion === null || javaMajorVersion >= 21) {
		args.push('-XX:G1UncommitBias=1')
	}
	return args.join(' ')
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
	return args.includes('-XX:+UseG1GC') && args.includes('-XX:MaxGCPauseMillis=0')
}

function detectG1gcPcl(args: string): boolean {
	return args.includes('-XX:+UseG1GC') && args.includes('-XX:MaxGCPauseMillis=50')
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
		baseArgs: buildG1gcMojangArgs(null),
		detect: detectG1gcMojang,
		buildArgs: (context) => buildG1gcMojangArgs(context?.javaMajorVersion ?? null),
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

/**
 * The preferred strategy plus the fallback chain, ordered by preference. The
 * backend verifies each block against the actual JVM and picks the first one
 * that is accepted (pruning unsupported tuning flags along the way).
 *
 * Fallbacks only ever move to less resource-hungry strategies — if the
 * heuristic deliberately avoided ZGC (insufficient resources), we must not
 * silently jump back up to it when Shenandoah is unavailable.
 */
const SAFE_TO_DEMANDING: ResolvedGcStrategyId[] = [
	'g1gc-pcl',
	'g1gc-mojang',
	'shenandoah',
	'zgc',
]

export function buildGcCandidateChain(
	context: GcContext,
	preferred: ResolvedGcStrategyId,
): { ids: string[]; args: string[][] } {
	const preferredDemand = SAFE_TO_DEMANDING.indexOf(preferred)
	const ids: string[] = [preferred]
	if (preferredDemand > 0) {
		for (let demand = preferredDemand - 1; demand >= 0; demand -= 1) {
			ids.push(SAFE_TO_DEMANDING[demand])
		}
	}
	// Absolute last resort: just the G1 selector (known to every HotSpot JVM).
	ids.push('minimal-g1')
	const args = ids.map((id) => {
		if (id === 'minimal-g1') return ['-XX:+UseG1GC']
		return GC_STRATEGY_DEFINITIONS[id].buildArgs(context).split(/\s+/).filter(Boolean)
	})
	return { ids, args }
}
