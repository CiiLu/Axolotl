import { get_version_many } from './cache.js'
import { getCurseForgeFile } from './curseforge'

const cache = new Map<string, { version: string; channel?: string; changelog?: string | null }>()

export function upgradeVersionCacheKey(provider: string, projectId: string, releaseId: string) {
	return `${provider}:${projectId}:${releaseId}`
}

export async function loadUpgradeVersionMetadata(
	provider: string,
	projectId: string,
	releaseId: string,
) {
	const key = upgradeVersionCacheKey(provider, projectId, releaseId)
	const cached = cache.get(key)
	if (cached) return cached
	if (provider === 'modrinth') {
		const versions = (await get_version_many([releaseId])) as Array<{
			id: string
			version_number?: string
			version_type?: string
			changelog?: string | null
		}>
		const version = versions[0]
		const result = {
			version: version?.version_number ?? releaseId,
			channel: version?.version_type,
			changelog: version?.changelog ?? null,
		}
		cache.set(key, result)
		return result
	}
	if (provider === 'curseforge') {
		const file = await getCurseForgeFile(Number(projectId), Number(releaseId))
		const result = {
			version: file.displayName ?? file.fileName ?? releaseId,
			channel: file.releaseType,
			changelog: null,
		}
		cache.set(key, result)
		return result
	}
	return { version: releaseId, changelog: null }
}
