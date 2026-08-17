import type { InstanceLoader } from '@/helpers/types'

export type GcStrategyId = 'g1gc-mojang' | 'g1gc-pcl' | 'shenandoah' | 'zgc' | 'auto'

export type ResolvedGcStrategyId = Exclude<GcStrategyId, 'auto'>

export interface GcContext {
	javaMajorVersion: number | null
	allocatedMemoryMb: number
	systemCpuCores: number
	systemLogicalProcessors: number
	modCount: number
	loader: InstanceLoader
}

export interface GcResolution {
	resolvedStrategy: ResolvedGcStrategyId
	reasonChain: string[]
}

export interface GcStrategyDefinition {
	id: GcStrategyId
	baseArgs: string
	detect: (currentArgs: string) => boolean
	buildArgs: (context?: GcContext) => string
}
