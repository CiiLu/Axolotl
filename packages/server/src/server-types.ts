import type {
	ResolveServerJarInput,
	ServerJarDownload,
	ServerTypeDefinition,
	ServerTypeId,
} from './types.ts'

const FABRIC_META_URL = 'https://meta.fabricmc.net/v2'
const PAPER_API_URL = 'https://api.papermc.io/v2'
const PAPER_PROJECT = 'paper'

/**
 * Known server types. `forge`, `neoforge` and `quilt` require an installer run
 * step that is not implemented yet (TODO) but are registered so the UI and
 * future CLI share one source of truth.
 */
export const SERVER_TYPES: Record<ServerTypeId, ServerTypeDefinition> = {
	vanilla: {
		id: 'vanilla',
		label: 'Vanilla',
		installMode: 'direct',
		needsLoaderVersion: false,
	},
	fabric: {
		id: 'fabric',
		label: 'Fabric',
		installMode: 'direct',
		needsLoaderVersion: true,
	},
	paper: {
		id: 'paper',
		label: 'Paper',
		installMode: 'direct',
		needsLoaderVersion: false,
	},
	forge: {
		id: 'forge',
		label: 'Forge',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
	neoforge: {
		id: 'neoforge',
		label: 'NeoForge',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
	quilt: {
		id: 'quilt',
		label: 'Quilt',
		installMode: 'installer',
		needsLoaderVersion: true,
	},
}

export function listServerTypes(): ServerTypeDefinition[] {
	return Object.values(SERVER_TYPES)
}

export function isServerTypeSupported(type: ServerTypeId): boolean {
	return SERVER_TYPES[type].installMode !== 'installer'
}

/** URL of the Fabric server launcher jar for a specific game/loader/installer combination. */
export function fabricServerJarUrl(
	gameVersion: string,
	loaderVersion: string,
	installerVersion: string,
): string {
	return `${FABRIC_META_URL}/versions/loader/${gameVersion}/${loaderVersion}/${installerVersion}/jar`
}

export function fabricInstallerVersionsUrl(): string {
	return `${FABRIC_META_URL}/versions/installer`
}

export function fabricLoaderVersionsForGameUrl(gameVersion: string): string {
	return `${FABRIC_META_URL}/versions/loader/${gameVersion}`
}

export function paperBuildsUrl(gameVersion: string): string {
	return `${PAPER_API_URL}/projects/${PAPER_PROJECT}/versions/${gameVersion}/builds`
}

export function paperDownloadUrl(gameVersion: string, build: number, filename: string): string {
	return `${PAPER_API_URL}/projects/${PAPER_PROJECT}/versions/${gameVersion}/builds/${build}/downloads/${filename}`
}

/**
 * Resolves the server jar download for a server type from metadata the caller
 * fetched. Returns null when the type needs an installer step or required
 * metadata is missing.
 */
export function resolveServerJar(
	type: ServerTypeId,
	input: ResolveServerJarInput,
): ServerJarDownload | null {
	switch (type) {
		case 'vanilla': {
			const server = input.vanillaVersionInfo?.downloads.server
			if (!server) return null
			return { url: server.url, filename: 'server.jar', sha1: server.sha1, size: server.size }
		}
		case 'fabric': {
			if (!input.loaderVersion) return null
			const installer = input.installerVersion ?? 'latest'
			return {
				url: fabricServerJarUrl(input.gameVersion, input.loaderVersion, installer),
				filename: 'fabric-server.jar',
			}
		}
		case 'paper': {
			if (!input.paperBuild) return null
			return {
				url: paperDownloadUrl(input.gameVersion, input.paperBuild.build, input.paperBuild.filename),
				filename: 'server.jar',
			}
		}
		default:
			return null
	}
}

export interface PaperBuildsResponse {
	builds: Array<{ build: number; downloads: { application: { name: string; sha256: string } } }>
}

export function latestPaperBuild(response: PaperBuildsResponse): {
	build: number
	filename: string
	sha256: string
} | null {
	const latest = response.builds?.[response.builds.length - 1]
	if (!latest?.downloads?.application) return null
	return {
		build: latest.build,
		filename: latest.downloads.application.name,
		sha256: latest.downloads.application.sha256,
	}
}

export interface FabricLoaderVersionsResponse {
	loader: Array<{ version: string; stable: boolean }>
}

export function pickFabricLoaderVersion(response: FabricLoaderVersionsResponse): string | null {
	return response.loader?.[0]?.version ?? null
}

/** Minimum Java major version required to run a given game version. */
export function requiredJavaMajorVersion(gameVersion: string): number {
	const match = /(\d+)\.(\d+)(?:\.(\d+))?/.exec(gameVersion)
	if (!match) return 21
	const minor = Number(match[2])
	const patch = Number(match[3] ?? 0)
	if (minor > 20 || (minor === 20 && patch >= 5)) return 21
	if (minor >= 17) return 17
	if (minor >= 12) return 17
	return 8
}
