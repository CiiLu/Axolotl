import type { InstanceLoader } from '@/helpers/types'

import type { GcContext } from './types'

export async function collectGcContext(
	allocatedMemoryMb: number,
	loader: InstanceLoader,
): Promise<GcContext> {
	const systemCpuCores = navigator.hardwareConcurrency ?? 4
	const systemLogicalProcessors = systemCpuCores

	return {
		javaMajorVersion: null,
		allocatedMemoryMb,
		systemCpuCores,
		systemLogicalProcessors,
		modCount: 0,
		loader,
	}
}
