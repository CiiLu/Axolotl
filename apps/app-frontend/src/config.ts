const trimTrailingSlash = (url: string) => url.replace(/\/$/, '')

export const AxolotlBrandConfig = Object.freeze({
	productName: 'Axolotl Launcher',
	shortProductName: 'Axolotl',
	website: 'https://www.axlmc.org/',
	repositoryUrl: 'https://github.com/Mystic-Stars/Axolotl',
	supportUrl: 'https://github.com/Mystic-Stars/Axolotl/issues',
	qqGroupNumber: '955605306',
	qqChannelUrl: 'https://pd.qq.com/s/9nfp5rlz0',
	sponsorUrl: 'https://afdian.com/a/Mystic-Stars',
	bundleIdentifier: 'red.ghs.axolotl',
	deepLinkScheme: 'axolotl',
	userAgent: (version: string, os: string) => `garbage-human-studio/axolotl/${version} (${os})`,
	capabilities: Object.freeze({
		publicModrinthApi: true,
		privateModrinthServices: false,
		ghsTelemetry: false,
	}),
})

const siteUrl = trimTrailingSlash(import.meta.env.MODRINTH_URL || 'https://modrinth.com')
const officialLabrinthBaseUrl = trimTrailingSlash(
	import.meta.env.MODRINTH_API_BASE_URL || 'https://api.modrinth.com',
)
export const MODRINTH_MIRROR_BASE_URL = 'https://mod.mcimirror.top/modrinth'
type DownloadSourceMode = 'auto' | 'official_only' | 'mirror_preferred' | 'official_preferred'

let modrinthSourceMode: DownloadSourceMode = 'auto'

function autoPrefersMirror() {
	if (typeof navigator === 'undefined') return false

	const languages = [...(navigator.languages ?? []), navigator.language]
	const usesMainlandChinese = languages.some((language) => {
		const normalized = language.toLowerCase().replace('_', '-')
		return normalized.startsWith('zh-cn') || normalized.startsWith('zh-hans')
	})
	const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone?.toLowerCase()
	const usesMainlandTimeZone = [
		'asia/shanghai',
		'asia/chongqing',
		'asia/harbin',
		'asia/urumqi',
	].includes(timeZone ?? '')

	return usesMainlandTimeZone || (!timeZone && usesMainlandChinese)
}

export function setModrinthSourceMode(sourceMode: DownloadSourceMode) {
	modrinthSourceMode = sourceMode
}

export function setModrinthMirrorEnabled(enabled: boolean) {
	setModrinthSourceMode(enabled ? 'mirror_preferred' : 'official_only')
}

export function getOfficialLabrinthBaseUrl() {
	return officialLabrinthBaseUrl
}

export function getLabrinthBaseUrl() {
	const useMirror =
		modrinthSourceMode === 'mirror_preferred' ||
		(modrinthSourceMode === 'auto' && autoPrefersMirror())
	return useMirror ? MODRINTH_MIRROR_BASE_URL : officialLabrinthBaseUrl
}

export const config = {
	siteUrl,
	labrinthBaseUrl: getLabrinthBaseUrl,
}
