import type { UpgradeFlowSnapshot } from '@/pages/instance/upgrade/flow'

let parked: UpgradeFlowSnapshot | null = null

export function parkUpgradeFlow(snapshot: UpgradeFlowSnapshot) {
	parked = structuredClone(snapshot)
}

export function peekUpgradeFlow(instanceId?: string): UpgradeFlowSnapshot | null {
	if (!parked || (instanceId && parked.instanceId !== instanceId)) return null
	return structuredClone(parked)
}

export function consumeUpgradeFlow(instanceId: string, returnFullPath: string): UpgradeFlowSnapshot | null {
	if (!parked || parked.instanceId !== instanceId || parked.returnFullPath !== returnFullPath) return null
	const snapshot = structuredClone(parked)
	parked = null
	return snapshot
}

export function clearUpgradeFlow() {
	parked = null
}

export function upgradeProjectPath(provider: string | null, projectId: string | null): string | null {
	if (!projectId) return null
	if (provider === 'modrinth') return `/project/${encodeURIComponent(projectId)}`
	if (provider === 'curseforge') return `/project/curseforge/${encodeURIComponent(projectId)}`
	return null
}
