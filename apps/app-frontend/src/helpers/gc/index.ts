export { GC_STRATEGY_DEFINITIONS, detectGcStrategy, getStrategyBaseArgs } from './strategies'
export { resolveAutoGcStrategy, getResolvedStrategyName } from './auto-selector'
export { collectGcContext } from './context'
export { createGcPresets, getAutoResolution, getResolvedStrategyDisplayName } from './gc-presets'
export type {
	GcContext,
	GcResolution,
	GcStrategyId,
	GcStrategyDefinition,
	ResolvedGcStrategyId,
} from './types'
