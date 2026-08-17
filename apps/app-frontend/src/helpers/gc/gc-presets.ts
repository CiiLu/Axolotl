import { defineMessage } from '@modrinth/ui'

import { getResolvedStrategyName, resolveAutoGcStrategy } from '@/helpers/gc/auto-selector'
import { detectGcStrategy, GC_STRATEGY_DEFINITIONS } from '@/helpers/gc/strategies'
import type { GcContext, JavaArgumentPreset } from '@/helpers/gc/types'

const GC_WIKI_URL = 'https://docs.oracle.com/en/java/javase/21/gctuning/introduction.html'
const G1GC_DOCS_URL =
	'https://docs.oracle.com/en/java/javase/21/gctuning/garbage-collector-implementation.html'
const SHENANDOAH_DOCS_URL = 'https://wiki.openjdk.org/display/shenandoah/Main'
const ZGC_DOCS_URL = 'https://wiki.openjdk.org/display/zgc/Main'

function buildAutoArgs(context: GcContext | undefined): string {
	if (!context) {
		return GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs
	}
	const resolution = resolveAutoGcStrategy(context)
	const strategy = GC_STRATEGY_DEFINITIONS[resolution.resolvedStrategy]
	return strategy.buildArgs(context)
}

function detectAuto(currentArgs: string): boolean {
	const detected = detectGcStrategy(currentArgs)
	return detected !== null
}

export function createGcPresets(gcContext?: GcContext): JavaArgumentPreset[] {
	const autoResolution = gcContext ? resolveAutoGcStrategy(gcContext) : null
	const autoResolvedName = autoResolution
		? getResolvedStrategyName(autoResolution.resolvedStrategy)
		: 'Mojang G1GC'

	return [
		{
			id: 'gc-g1gc-mojang',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-mojang.title',
				defaultMessage: 'Mojang G1GC',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-mojang.description',
				defaultMessage: 'G1GC tuning used by the official Minecraft launcher',
			}),
			args: GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS['g1gc-mojang'].baseArgs,
			detect: GC_STRATEGY_DEFINITIONS['g1gc-mojang'].detect,
			link: G1GC_DOCS_URL,
		},
		{
			id: 'gc-g1gc-pcl',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-pcl.title',
				defaultMessage: 'PCL G1GC',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.g1gc-pcl.description',
				defaultMessage: 'G1GC tuning optimized by PCL launcher',
			}),
			args: GC_STRATEGY_DEFINITIONS['g1gc-pcl'].baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS['g1gc-pcl'].baseArgs,
			detect: GC_STRATEGY_DEFINITIONS['g1gc-pcl'].detect,
			link: G1GC_DOCS_URL,
		},
		{
			id: 'gc-shenandoah',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.shenandoah.title',
				defaultMessage: 'Shenandoah',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.shenandoah.description',
				defaultMessage: 'Low-pause GC for medium-to-heavy modpacks (Java 12+)',
			}),
			args: GC_STRATEGY_DEFINITIONS.shenandoah.baseArgs,
			resolveArgs: () => GC_STRATEGY_DEFINITIONS.shenandoah.baseArgs,
			detect: GC_STRATEGY_DEFINITIONS.shenandoah.detect,
			link: SHENANDOAH_DOCS_URL,
		},
		{
			id: 'gc-zgc',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.zgc.title',
				defaultMessage: 'ZGC',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.zgc.description',
				defaultMessage: 'Ultra-low latency GC for high-end systems (Java 15+)',
			}),
			args: GC_STRATEGY_DEFINITIONS.zgc.buildArgs(gcContext),
			resolveArgs: (context) => GC_STRATEGY_DEFINITIONS.zgc.buildArgs(context),
			detect: GC_STRATEGY_DEFINITIONS.zgc.detect,
			link: ZGC_DOCS_URL,
		},
		{
			id: 'gc-auto',
			group: 'gc',
			title: defineMessage({
				id: 'app.java-arguments.presets.gc.auto.title',
				defaultMessage: 'Auto',
			}),
			description: defineMessage({
				id: 'app.java-arguments.presets.gc.auto.description',
				defaultMessage: 'Automatically select the best GC strategy for your system',
			}),
			args: buildAutoArgs(gcContext),
			resolveArgs: (context) => buildAutoArgs(context),
			detect: detectAuto,
			link: GC_WIKI_URL,
			autoResolvedName,
			autoReasonChain: autoResolution?.reasonChain,
		},
	]
}
