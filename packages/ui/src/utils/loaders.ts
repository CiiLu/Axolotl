import type { Archon } from '@modrinth/api-client'

export type ServerLoader = Archon.Servers.v0.Loader | 'Bukkit'

export const clientInstallableLoaders = [
	'fabric',
	'neoforge',
	'forge',
	'quilt',
	'optifine',
	'cleanroom',
	'lite_loader',
	'legacy_fabric',
] as const

export const instanceInstallablePlatforms = ['vanilla', ...clientInstallableLoaders] as const

export const loaderDisplayNames: Record<string, string> = {
	fabric: 'Fabric',
	neoforge: 'NeoForge',
	neo_forge: 'NeoForge',
	forge: 'Forge',
	quilt: 'Quilt',
	paper: 'Paper',
	spigot: 'Spigot',
	purpur: 'Purpur',
	bukkit: 'Bukkit',
	vanilla: 'Vanilla',
	lite_loader: 'LiteLoader',
	cleanroom: 'Cleanroom',
	legacy_fabric: 'Legacy Fabric',
	optifine: 'OptiFine',
}

export const loaderMessages: Record<string, { id: string; defaultMessage: string }> = {
	vanilla: {
		id: 'loader.vanilla',
		defaultMessage: 'None',
	},
}

export const formatLoaderLabel = (
	item: string,
	formatMessage?: (msg: { id: string; defaultMessage: string }) => string,
) => {
	if (formatMessage && loaderMessages[item]) {
		return formatMessage(loaderMessages[item])
	}
	return loaderDisplayNames[item] ?? item.charAt(0).toUpperCase() + item.slice(1)
}
