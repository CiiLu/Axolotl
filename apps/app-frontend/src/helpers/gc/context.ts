import { get_content_snapshot } from '@/helpers/instance'
import type { AppSettings } from '@/helpers/types'

import type { GcContext } from './types'
import type { InstanceLoader } from '@/helpers/types'

interface InstanceLike {
	id: string
	loader: InstanceLoader
	java_path?: string
	memory?: { maximum: number; automatic: boolean }
}

function extractJavaMajorVersion(parsedVersion: string | null | undefined): number | null {
	if (!parsedVersion) return null
	const match = parsedVersion.match(/(?:^|\.)(\d+)(?:\.|$)/)
	if (!match) return null
	const num = parseInt(match[1], 10)
	if (Number.isNaN(num)) return null
	if (num === 1) {
		const minorMatch = parsedVersion.match(/^1\.(\d+)/)
		return minorMatch ? parseInt(minorMatch[1], 10) : null
	}
	return num
}

export async function collectGcContext(
	instance: InstanceLike,
	globalSettings: AppSettings | null,
	getOptimalJavaVersion: () => Promise<string | null>,
): Promise<GcContext> {
	let javaMajorVersion: number | null = null
	try {
		const optimalJavaVersion = await getOptimalJavaVersion()
		javaMajorVersion = extractJavaMajorVersion(optimalJavaVersion)
	} catch {
		javaMajorVersion = null
	}

	const allocatedMemoryMb =
		instance.memory?.maximum ?? globalSettings?.memory?.maximum ?? 2048

	const systemCpuCores = navigator.hardwareConcurrency ?? 4
	const systemLogicalProcessors = systemCpuCores

	let modCount = 0
	try {
		const snapshot = await get_content_snapshot(instance.id)
		modCount = snapshot.items.filter(
			(item) => item.projectType === 'mod' && item.materializationState === 'present',
		).length
	} catch {
		modCount = 0
	}

	return {
		javaMajorVersion,
		allocatedMemoryMb,
		systemCpuCores,
		systemLogicalProcessors,
		modCount,
		loader: instance.loader,
	}
}
