<script setup lang="ts">
import AppleIcon from '@modrinth/assets/external/apple.svg?component'
import LinuxIcon from '@modrinth/assets/external/linux.svg?component'
import WindowsIcon from '@modrinth/assets/external/windows.svg?component'
import ArrowDownIcon from '@modrinth/assets/icons/arrow-down.svg?component'
import CompassIcon from '@modrinth/assets/icons/compass.svg?component'
import DownloadIcon from '@modrinth/assets/icons/download.svg?component'
import EyeOffIcon from '@modrinth/assets/icons/eye-off.svg?component'
import IssuesIcon from '@modrinth/assets/icons/issues.svg?component'
import LanguagesIcon from '@modrinth/assets/icons/languages.svg?component'
import SearchIcon from '@modrinth/assets/icons/search.svg?component'
import TrashIcon from '@modrinth/assets/icons/trash.svg?component'
import Accordion from '@modrinth/ui/src/components/base/Accordion.vue'
import Avatar from '@modrinth/ui/src/components/base/Avatar.vue'
import ButtonStyled from '@modrinth/ui/src/components/base/ButtonStyled.vue'
import Checkbox from '@modrinth/ui/src/components/base/Checkbox.vue'
import IntlFormatted from '@modrinth/ui/src/components/base/IntlFormatted.vue'
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

import AppleLogo from '~/components/landing/AppleLogo.vue'
import LinuxLogo from '~/components/landing/LinuxLogo.vue'
import MultiplayerIcon from '~/components/landing/MultiplayerIcon.vue'
import OfflineModeIcon from '~/components/landing/OfflineModeIcon.vue'
import OpenSourceIcon from '~/components/landing/OpenSourceIcon.vue'
import WindowsLogo from '~/components/landing/WindowsLogo.vue'

interface GitHubReleaseAsset {
	browser_download_url: string
	name: string
}

interface GitHubRelease {
	assets: GitHubReleaseAsset[]
	tag_name: string
}

type OSType = 'Mac' | 'Windows' | 'Linux' | null

const downloadWindows = ref<HTMLAnchorElement | null>(null)
const downloadMac = ref<HTMLAnchorElement | null>(null)
const downloadSection = ref<HTMLElement | null>(null)
const hero = ref<HTMLElement | null>(null)

const updateHeroTilt = (event: PointerEvent) => {
	if (!hero.value) return

	const bounds = hero.value.getBoundingClientRect()
	const x = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width))
	const y = Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height))

	hero.value.style.setProperty('--pointer-x', `${x * 100}%`)
	hero.value.style.setProperty('--pointer-y', `${y * 100}%`)
	hero.value.style.setProperty('--tilt-x', `${(0.5 - y) * 4}deg`)
	hero.value.style.setProperty('--tilt-y', `${(x - 0.5) * 5}deg`)
}

const resetHeroTilt = () => {
	if (!hero.value) return

	hero.value.style.removeProperty('--pointer-x')
	hero.value.style.removeProperty('--pointer-y')
	hero.value.style.removeProperty('--tilt-x')
	hero.value.style.removeProperty('--tilt-y')
}

const { resolvedSource } = useDownloadSource()
const CNB_RELEASE_BASE_URL = 'https://cnb.cool/axlmc/Axolotl/-/releases/download'

const windowsLink = ref<string | null>(null)

const linuxLinks = reactive({
	appImage: null as string | null,
	deb: null as string | null,
	rpm: null as string | null,
})

const macLinks = reactive({
	universal: null as string | null,
})

const { data: launcherRelease } = await useFetch<GitHubRelease>(
	'https://api.github.com/repos/Mystic-Stars/Axolotl/releases/latest',
	{
		server: false,
		getCachedData(key, nuxtApp) {
			const cached = (nuxtApp.ssrContext?.cache as any)?.[key] || nuxtApp.payload.data[key]
			if (!cached) return

			const now = Date.now()
			const cacheTime = cached._cacheTime || 0
			const maxAge = 5 * 60 * 1000

			if (now - cacheTime > maxAge) {
				return null
			}

			return cached
		},
		transform(data) {
			return {
				...data,
				_cacheTime: Date.now(),
			}
		},
	},
)

const platform = computed<string>(() => {
	if (import.meta.server) {
		const headers = useRequestHeaders()
		return headers['user-agent'] || ''
	} else {
		return navigator.userAgent || ''
	}
})
const os = computed<OSType>(() => {
	if (platform.value.includes('Mac')) {
		return 'Mac'
	} else if (platform.value.includes('Win')) {
		return 'Windows'
	} else if (platform.value.includes('Linux')) {
		return 'Linux'
	} else {
		return null
	}
})

const modManagementData = [
	{
		id: 'P7dR8mSH', // Todo: fetch name + author + icon from api
		name: 'Fabric API',
		author: 'modmuss50',
		version: '0.86.1+1.20.1',
		iconUrl: 'https://cdn.modrinth.com/data/P7dR8mSH/icon.png',
	},
	{
		id: 'AANobbMI',
		name: 'Sodium',
		author: 'jellysquid3',
		version: 'mc1.20.1-0.5.0',
		iconUrl: 'https://cdn.modrinth.com/data/AANobbMI/icon.png',
	},
	{
		id: 'YL57xq9U',
		name: 'Iris Shaders',
		author: 'coderbot',
		version: '1.6.5+1.20.1',
		iconUrl: 'https://cdn.modrinth.com/data/YL57xq9U/dc558eece920db435f9823ce86de0c4cde89800b.png',
	},
	{
		id: 'gvQqBUqZ',
		name: 'Lithium',
		author: 'jellysquid3',
		version: 'mc1.20.1-0.11.2',
		iconUrl: 'https://cdn.modrinth.com/data/gvQqBUqZ/icon.png',
	},
	{
		id: 'mOgUt4GM',
		name: 'Mod Menu',
		author: 'Prospector',
		version: '7.2.1',
		iconUrl:
			'https://cdn.modrinth.com/data/mOgUt4GM/1bfe2006b38340e9d064700e41adf84a8abb1bd4_96.webp',
	},
	{
		id: '9s6osm5g',
		name: 'Cloth Config API',
		author: 'shedaniel',
		version: '11.1.106+fabric',
		iconUrl: 'https://cdn.modrinth.com/data/9s6osm5g/icon.png',
	},
	{
		id: 'lhGA9TYQ',
		name: 'Architectury API',
		author: 'shedaniel',
		version: '9.1.12+fabric',
		iconUrl: 'https://cdn.modrinth.com/data/lhGA9TYQ/icon.png',
	},
	{
		id: 'nrJ2NpD0',
		name: 'Craftify',
		author: 'ThatGravyBoat',
		version: '8.5.2023',
		iconUrl: 'https://cdn.modrinth.com/data/nrJ2NpD0/4f21214db060ed4542b1f3983c4113d293480a1b.webp',
	},
]

const newProjects = Array.from({ length: 40 }, (_, index) => {
	const project = modManagementData[index % modManagementData.length]

	return {
		id: `${project.id}-${index}`,
		icon_url: project.iconUrl,
		title: project.name,
		description: `${project.name} is available through Axolotl's content browser.`,

		// 第一轮是真实语义内容，后面的只是视觉重复
		isVisualDuplicate: index >= modManagementData.length,
	}
})
const rowCount = 5
const perRow = Math.ceil(newProjects.length / rowCount)
const rows = Array.from({ length: rowCount }, (_, index) =>
	newProjects.slice(index * perRow, (index + 1) * perRow),
)

const downloadLauncher = computed(() => {
	if (os.value === 'Windows') {
		return () => {
			downloadWindows.value?.click()
		}
	} else if (os.value === 'Mac') {
		return () => {
			downloadMac.value?.click()
		}
	} else {
		return () => {
			scrollToSection()
		}
	}
})

const handleDownload = () => {
	downloadLauncher.value()
}

watch(
	[launcherRelease, resolvedSource],
	([release]) => {
		const findAsset = (patterns: RegExp[]) => {
			const asset = release?.assets.find((item) =>
				patterns.some((pattern) => pattern.test(item.name)),
			)
			if (!asset) return null

			if (resolvedSource.value === 'cnb') {
				return `${CNB_RELEASE_BASE_URL}/${encodeURIComponent(release.tag_name)}/${encodeURIComponent(asset.name)}`
			}

			return asset.browser_download_url
		}

		windowsLink.value = findAsset([/x64-setup\.exe$/i, /\.exe$/i])
		macLinks.universal = findAsset([/universal\.dmg$/i, /\.dmg$/i])
		linuxLinks.appImage = findAsset([/(amd64|x86_64)\.AppImage$/i])
		linuxLinks.deb = findAsset([/_amd64\.deb$/i, /(amd64|x86_64).*\.deb$/i])
		linuxLinks.rpm = findAsset([/(x86_64|amd64).*\.rpm$/i])
	},
	{ immediate: true },
)

const scrollToSection = () => {
	nextTick(() => {
		if (downloadSection.value) {
			window.scrollTo({
				top: downloadSection.value.offsetTop,
				behavior: 'smooth',
			})
		}
	})
}

const { formatMessage, locale } = useVIntl()

const messages = defineMessages({
	openSourceBadge: {
		id: 'axolotl-marketing.hero.open-source',
		defaultMessage: 'Tauri v2 - Rust - Vue 3',
	},
	oneLauncher: {
		id: 'axolotl-marketing.demo.one-launcher',
		defaultMessage: 'One launcher. Every world.',
	},
	everythingTogether: {
		id: 'axolotl-marketing.demo.everything-together',
		defaultMessage: 'Profiles, mods, saves, and settings stay together.',
	},
	includedMods: {
		id: 'axolotl-marketing.demo.included-mods',
		defaultMessage: 'Included mods',
	},
	downloadAxolotl: {
		id: 'axolotl-marketing.hero.download',
		defaultMessage: 'Axolotl Launcher',
	},
	downloadAxolotlForOs: {
		id: 'axolotl-marketing.hero.download-for-os',
		defaultMessage: 'Axolotl Launcher for {os}',
	},
	description: {
		id: 'app-marketing.hero.description',
		defaultMessage:
			'Axolotl Launcher is a free, open-source, ad-free, cross-platform Minecraft Java Edition launcher for searching, installing, and updating mods, modpacks, resource packs, and shaders from Modrinth and CurseForge, with Axolotl Labs built in.',
	},
	heroScreenshotAlt: {
		id: 'axolotl-marketing.hero.screenshot-alt',
		defaultMessage: 'Axolotl Launcher home screen.',
	},
	builtOnModrinth: {
		id: 'axolotl-marketing.highlights.eyebrow',
		defaultMessage: 'One launcher, two sources',
	},
	highlightsTitle: {
		id: 'axolotl-marketing.highlights.title',
		defaultMessage: 'Manage Minecraft content',
	},
	highlightsTitleSecond: {
		id: 'axolotl-marketing.highlights.title-second',
		defaultMessage: 'without detours',
	},
	highlightsDescription: {
		id: 'axolotl-marketing.highlights.description',
		defaultMessage:
			'Search Modrinth and CurseForge, then inspect projects, choose versions, install content, resolve dependencies, and keep it updated from the launcher.',
	},
	modrinthNative: {
		id: 'axolotl-marketing.highlights.modrinth.title',
		defaultMessage: 'Modrinth and CurseForge',
	},
	modrinthNativeDescription: {
		id: 'axolotl-marketing.highlights.modrinth.description',
		defaultMessage:
			'Discover mods, modpacks, resource packs, shaders, and more from both sources without leaving your launcher.',
	},
	adFree: {
		id: 'axolotl-marketing.highlights.ad-free.title',
		defaultMessage: 'Free, open, and independent',
	},
	adFreeDescription: {
		id: 'axolotl-marketing.highlights.ad-free.description',
		defaultMessage:
			'GPL-3.0, free to use, and ad-free. Axolotl is not an official Modrinth client.',
	},
	localized: {
		id: 'axolotl-marketing.highlights.localized.title',
		defaultMessage: 'Content management that stays organized',
	},
	localizedDescription: {
		id: 'axolotl-marketing.highlights.localized.description',
		defaultMessage:
			'Install and manage modpacks alongside individual projects. Some CurseForge files have distribution limits and may require a manual download.',
	},
	offlineAccounts: {
		id: 'axolotl-marketing.showcase.offline.title',
		defaultMessage: 'Accounts on your terms',
	},
	offlineAccountsDescription: {
		id: 'axolotl-marketing.showcase.offline.description',
		defaultMessage:
			'Sign in with Microsoft, create a local offline identity, or use Yggdrasil authentication with LittleSkin presets and custom servers.',
	},
	offlineLabel: { id: 'axolotl-marketing.showcase.offline.label', defaultMessage: 'Accounts' },
	themes: {
		id: 'axolotl-marketing.showcase.themes.title',
		defaultMessage: 'A color theme for every setup',
	},
	themesDescription: {
		id: 'axolotl-marketing.showcase.themes.description',
		defaultMessage:
			'Switch between light, dark, OLED, and system modes, then set your accent color, background, and transparency to match your setup.',
	},
	personalizeLabel: {
		id: 'axolotl-marketing.showcase.themes.label',
		defaultMessage: 'Personalize',
	},
	translation: {
		id: 'axolotl-marketing.showcase.translation.title',
		defaultMessage: 'Axolotl Lab, inside the launcher',
	},
	translationDescription: {
		id: 'axolotl-marketing.showcase.translation.description',
		defaultMessage:
			'Use the gradient text generator, Java Edition seed map, and 3D schematic workshop directly in Axolotl, not through external web pages.',
	},
	translateLabel: { id: 'axolotl-marketing.showcase.translation.label', defaultMessage: 'Lab' },
	offlineScreenshotAlt: {
		id: 'axolotl-marketing.showcase.offline.alt',
		defaultMessage: 'Axolotl Launcher offline account dialog.',
	},
	themesScreenshotAlt: {
		id: 'axolotl-marketing.showcase.themes.alt',
		defaultMessage: 'Axolotl Launcher theme customization settings.',
	},
	translationScreenshotAlt: {
		id: 'axolotl-marketing.showcase.translation.alt',
		defaultMessage: 'Axolotl Launcher Lab.',
	},
	downloadAxolotlButton: {
		id: 'axolotl-marketing.hero.download-button',
		defaultMessage: 'Download Axolotl',
	},
	moreDownloadOptions: {
		id: 'app-marketing.hero.more-download-options',
		defaultMessage: 'More Download Options',
	},
	unlikeAnyLauncher: {
		id: 'app-marketing.features.unlike-any-launcher',
		defaultMessage: 'Unlike any launcher',
	},
	youveUsedBefore: {
		id: 'app-marketing.features.youve-used-before',
		defaultMessage: "you've used before",
	},
	installedMods: {
		id: 'app-marketing.features.mod-management.installed-mods',
		defaultMessage: 'Installed mods',
	},
	searchMods: {
		id: 'app-marketing.features.mod-management.search-mods',
		defaultMessage: 'Search mods',
	},
	name: {
		id: 'app-marketing.features.mod-management.name',
		defaultMessage: 'Name',
	},
	version: {
		id: 'app-marketing.features.mod-management.version',
		defaultMessage: 'Version',
	},
	actions: {
		id: 'app-marketing.features.mod-management.actions',
		defaultMessage: 'Actions',
	},
	byAuthor: {
		id: 'app-marketing.features.mod-management.byAuthor',
		defaultMessage: 'by {author}',
	},
	modManagement: {
		id: 'app-marketing.features.mod-management.title',
		defaultMessage: 'Efficient instance management',
	},
	modManagementDescription: {
		id: 'app-marketing.features.mod-management.description',
		defaultMessage:
			'Create, import, and manage instances in bulk. Keep mods, resource packs, shaders, files, worlds, screenshots, and logs together with updates, launch settings, and modpack export.',
	},
	playWithFavoriteMods: {
		id: 'app-marketing.features.play.title',
		defaultMessage: 'Play with your favorite mods',
	},
	playWithFavoriteModsDescription: {
		id: 'app-marketing.features.play.description',
		defaultMessage:
			'Build an instance, add your favorite content, and jump into Minecraft in a few clicks.',
	},
	shareModpacks: {
		id: 'app-marketing.features.sharing.title',
		defaultMessage: 'Share modpacks',
	},
	shareModpacksDescription: {
		id: 'app-marketing.features.sharing.description',
		defaultMessage:
			'Build, export, and share modpacks while keeping every dependency and version together.',
	},
	share: {
		id: 'app-marketing.features.sharing.share-button',
		defaultMessage: 'Share',
	},
	modpack: {
		id: 'app-marketing.features.sharing.modpack',
		defaultMessage: 'Modpack',
	},
	activityMonitor: {
		id: 'app-marketing.features.performance.activity-monitor',
		defaultMessage: 'Activity monitor',
	},
	goodPerformance: {
		id: 'app-marketing.features.performance.good-performance',
		defaultMessage: 'Good performance',
	},
	processName: {
		id: 'app-marketing.features.performance.process-name',
		defaultMessage: 'Process name',
	},
	cpuPercent: {
		id: 'app-marketing.features.performance.cpu-percent',
		defaultMessage: '% CPU',
	},
	ram: {
		id: 'app-marketing.features.performance.ram',
		defaultMessage: 'RAM',
	},
	axolotlApp: {
		id: 'axolotl-marketing.features.performance.axolotl-app',
		defaultMessage: 'Axolotl Launcher',
	},
	small: {
		id: 'app-marketing.features.performance.small',
		defaultMessage: 'Small',
	},
	lessThan150MB: {
		id: 'app-marketing.features.performance.less-than-150mb',
		defaultMessage: '< 150 MB',
	},
	googleChrome: {
		id: 'app-marketing.features.performance.google-chrome',
		defaultMessage: 'Google Chrome',
	},
	discord: {
		id: 'app-marketing.features.performance.discord',
		defaultMessage: 'Discord',
	},
	infiniteMB: {
		id: 'app-marketing.features.performance.infinite-mb',
		defaultMessage: '∞ MB',
	},
	oneBillionPercent: {
		id: 'app-marketing.features.performance.one-billion-percent',
		defaultMessage: '1 billion %',
	},
	infiniteTimesInfiniteMB: {
		id: 'app-marketing.features.performance.infinite-times-infinite-mb',
		defaultMessage: '∞ × ∞ MB',
	},
	performant: {
		id: 'app-marketing.features.performance.title',
		defaultMessage: 'Performant',
	},
	performantDescription: {
		id: 'app-marketing.features.performance.description',
		defaultMessage:
			'Axolotl stays out of your way with a responsive interface and a lightweight desktop core.',
	},
	websiteIntegration: {
		id: 'app-marketing.features.website.title',
		defaultMessage: 'Search both sources in one launcher',
	},
	websiteIntegrationDescription: {
		id: 'app-marketing.features.website.description',
		defaultMessage:
			'Use project details and version selection to move from Modrinth or CurseForge discovery to an installed instance, with dependencies and updates handled in place.',
	},
	profileImporting: {
		id: 'app-marketing.features.importing.title',
		defaultMessage: 'Profile importing',
	},
	profileImportingDescription: {
		id: 'app-marketing.features.importing.description',
		defaultMessage:
			'Bring your existing profiles from popular launchers and continue playing without rebuilding everything by hand.',
	},
	openSource: {
		id: 'app-marketing.features.open-source.title',
		defaultMessage: 'Tauri v2, built in the open',
	},
	openSourceDescription: {
		id: 'app-marketing.features.open-source.description',
		defaultMessage:
			'Axolotl uses a Tauri v2 desktop foundation instead of Electron. It is an independent, unofficial downstream client based on the Modrinth monorepo. Follow development on <github-link>GitHub</github-link>.',
	},
	offlineMode: {
		id: 'app-marketing.features.offline.title',
		defaultMessage: 'Useful around every world',
	},
	offlineModeDescription: {
		id: 'app-marketing.features.offline.description',
		defaultMessage:
			'Chinese search and project translation, drag-and-drop import, Java management, offline mode, and skin management are ready when you need them.',
	},
	followProjects: {
		id: 'app-marketing.features.follow.title',
		defaultMessage: 'Beta multiplayer support',
	},
	followProjectsDescription: {
		id: 'app-marketing.features.follow.description',
		defaultMessage: 'Try the launcher multiplayer features while they continue to develop.',
	},
	downloadOptions: {
		id: 'app-marketing.download.options-title',
		defaultMessage: 'Download options',
	},
	downloadAxolotlTitle: {
		id: 'axolotl-marketing.download.title',
		defaultMessage: 'Download Axolotl Launcher',
	},
	downloadDescription: {
		id: 'app-marketing.download.description',
		defaultMessage:
			'Our desktop app is available across all platforms, choose your desired version.',
	},
	windows: {
		id: 'app-marketing.download.windows',
		defaultMessage: 'Windows',
	},
	mac: {
		id: 'app-marketing.download.mac',
		defaultMessage: 'Mac',
	},
	linux: {
		id: 'app-marketing.download.linux',
		defaultMessage: 'Linux',
	},
	downloadInstaller: {
		id: 'axolotl-marketing.download.installer',
		defaultMessage: 'Download installer',
	},
	downloadAppImage: {
		id: 'axolotl-marketing.download.appimage',
		defaultMessage: 'Download the AppImage',
	},
	showOtherPackages: {
		id: 'app-marketing.show-other-packages',
		defaultMessage: 'Show other packages',
	},
	hideOtherPackages: {
		id: 'app-marketing.hide-other-packages',
		defaultMessage: 'Hide other packages',
	},
	notRecommended: {
		id: 'app-marketing.not-recommended',
		defaultMessage: 'Choose the package format that matches your Linux distribution.',
	},
	downloadTheDEB: {
		id: 'app-marketing.download.download-deb',
		defaultMessage: 'Download the DEB',
	},
	downloadTheRPM: {
		id: 'app-marketing.download.download-rpm',
		defaultMessage: 'Download the RPM',
	},
	downloadTerms: {
		id: 'app-marketing.download.terms',
		defaultMessage:
			'Axolotl is free software released under <terms-link>GPL-3.0</terms-link>. Read the <privacy-link>Privacy Policy</privacy-link> before installing.',
	},
	linuxDisclaimer: {
		id: 'app-marketing.download.linux-disclaimer',
		defaultMessage:
			'Linux packages are published with every release. Check the <issues-link>release page</issues-link> for architecture details or <prism-link>report an issue</prism-link> if your distribution needs extra setup.',
	},
	seoTitle: {
		id: 'axolotl-site.seo.title',
		defaultMessage: 'Axolotl Launcher - Free Open-Source Modrinth + Curseforge Minecraft Launcher',
	},
	seoDescription: {
		id: 'axolotl-site.seo.description',
		defaultMessage:
			'Download Axolotl Launcher, a free, open-source Tauri v2 Minecraft launcher for Windows, macOS, and Linux with Modrinth and CurseForge content management, themes, accounts, and more.',
	},
	socialImageAlt: {
		id: 'axolotl-site.seo.social-image-alt',
		defaultMessage: 'Axolotl Launcher showing a Minecraft instance and its installed content.',
	},
	faqEyebrow: {
		id: 'axolotl-site.faq.eyebrow',
		defaultMessage: 'Frequently asked questions',
	},
	faqTitle: {
		id: 'axolotl-site.faq.title',
		defaultMessage: 'Everything you need to know about Axolotl',
	},
	faqDescription: {
		id: 'axolotl-site.faq.description',
		defaultMessage: 'Learn about supported platforms, accounts, content, and downloads.',
	},
	faqPlatformsQuestion: {
		id: 'axolotl-site.faq.platforms.question',
		defaultMessage: 'Which operating systems does Axolotl Launcher support?',
	},
	faqPlatformsAnswer: {
		id: 'axolotl-site.faq.platforms.answer',
		defaultMessage:
			'Axolotl Launcher supports Windows 10 and 11 on x64, macOS on Intel and Apple Silicon, and Linux x64 through AppImage, DEB, and RPM packages.',
	},
	faqFreeQuestion: {
		id: 'axolotl-site.faq.free.question',
		defaultMessage: 'Is Axolotl Launcher free and open source?',
	},
	faqFreeAnswer: {
		id: 'axolotl-site.faq.free.answer',
		defaultMessage:
			'Yes. Axolotl Launcher is free software released under GPL-3.0. Its source code and release history are publicly available on GitHub.',
	},
	faqAccountsQuestion: {
		id: 'axolotl-site.faq.accounts.question',
		defaultMessage: 'Can I use Microsoft and offline Minecraft accounts?',
	},
	faqAccountsAnswer: {
		id: 'axolotl-site.faq.accounts.answer',
		defaultMessage:
			'Yes. Axolotl supports Microsoft Minecraft accounts, local offline accounts, and third-party Yggdrasil authentication, including LittleSkin presets and custom servers.',
	},
	faqContentQuestion: {
		id: 'axolotl-site.faq.content.question',
		defaultMessage: 'Where does Axolotl get mods and other Minecraft content?',
	},
	faqContentAnswer: {
		id: 'axolotl-site.faq.content.answer',
		defaultMessage:
			'Axolotl helps you search, inspect, choose versions for, install, update, and manage content from Modrinth and CurseForge. Files with CurseForge distribution restrictions may require a manual download.',
	},
	faqDownloadQuestion: {
		id: 'axolotl-site.faq.download.question',
		defaultMessage: 'Where should I download Axolotl Launcher?',
	},
	faqDownloadAnswer: {
		id: 'axolotl-site.faq.download.answer',
		defaultMessage:
			'Use the download section on this official website. Automatic mode selects CNB for mainland China and GitHub elsewhere; you can change the source in website settings.',
	},
	appScreenshotAlt: {
		id: 'app-marketing.hero.app-screenshot-alt',
		defaultMessage: `Axolotl Launcher instance content preview.`,
	},
	gdlauncherAlt: {
		id: 'app-marketing.features.importing.gdlauncher-alt',
		defaultMessage: 'GDLauncher',
	},
	multimcAlt: {
		id: 'app-marketing.features.importing.multimc-alt',
		defaultMessage: 'MultiMC',
	},
	structuredFeatureContentSources: {
		id: 'axolotl-site.structured-data.feature.content-sources',
		defaultMessage:
			'Search, install, and update mods, modpacks, resource packs, and shaders from Modrinth and CurseForge',
	},
	structuredFeatureLab: {
		id: 'axolotl-site.structured-data.feature.lab',
		defaultMessage:
			'Axolotl Labs with gradient text generator, Java Edition seed map, and 3D schematic workshop',
	},
	structuredFeatureInstances: {
		id: 'axolotl-site.structured-data.feature.instances',
		defaultMessage: 'Instance, world, screenshot, log, Java, and modpack management',
	},
	structuredFeatureAccounts: {
		id: 'axolotl-site.structured-data.feature.accounts',
		defaultMessage: 'Microsoft, offline, LittleSkin, and custom Yggdrasil account support',
	},
	faqLabQuestion: {
		id: 'axolotl-site.faq.lab.question',
		defaultMessage: 'What is Axolotl Labs?',
	},
	faqLabAnswer: {
		id: 'axolotl-site.faq.lab.answer',
		defaultMessage:
			'Axolotl Labs is a collection of built-in launcher tools, including a gradient text generator, Java Edition seed map, and 3D schematic workshop.',
	},
	faqProjectDisclaimerQuestion: {
		id: 'axolotl-site.faq.project-disclaimer.question',
		defaultMessage: 'Is Axolotl Launcher and Axolotl Client the same project?',
	},
	faqProjectDisclaimerAnswer: {
		id: 'axolotl-site.faq.project-disclaimer.answer',
		defaultMessage:
			'No. Axolotl Launcher and Axolotl Client are separate projects. Axolotl Launcher is an independent, unofficial downstream launcher based on the Modrinth monorepo. It is not affiliated with other Minecraft projects named Axolotl Client.',
	},
	seoKeywords: {
		id: 'axolotl-site.seo.keywords',
		defaultMessage:
			'Axolotl Launcher, Minecraft Launcher, Modrinth, CurseForge, Minecraft Java Edition, Axolotl Labs',
	},
})

const config = useRuntimeConfig()
const siteUrl = config.public.siteUrl
const canonicalUrl = `${siteUrl}/`
const socialImageUrl = `${siteUrl}/showcase/launcher-home.png`
const githubUrl = 'https://github.com/Mystic-Stars/Axolotl'
const licenseUrl = `${githubUrl}/blob/main/LICENSE`

const title = computed(() => formatMessage(messages.seoTitle))
const description = computed(() => formatMessage(messages.seoDescription))
const socialImageAlt = computed(() => formatMessage(messages.socialImageAlt))
const faqItems = computed(() => [
	{
		question: formatMessage(messages.faqPlatformsQuestion),
		answer: formatMessage(messages.faqPlatformsAnswer),
	},
	{
		question: formatMessage(messages.faqFreeQuestion),
		answer: formatMessage(messages.faqFreeAnswer),
	},
	{
		question: formatMessage(messages.faqAccountsQuestion),
		answer: formatMessage(messages.faqAccountsAnswer),
	},
	{
		question: formatMessage(messages.faqContentQuestion),
		answer: formatMessage(messages.faqContentAnswer),
	},
	{
		question: formatMessage(messages.faqDownloadQuestion),
		answer: formatMessage(messages.faqDownloadAnswer),
	},
	{
		question: formatMessage(messages.faqLabQuestion),
		answer: formatMessage(messages.faqLabAnswer),
	},
	{
		question: formatMessage(messages.faqProjectDisclaimerQuestion),
		answer: formatMessage(messages.faqProjectDisclaimerAnswer),
	},
])
const keywords = computed(() => formatMessage(messages.seoKeywords))

const structuredData = computed(() => ({
	'@context': 'https://schema.org',
	'@graph': [
		{
			'@type': 'WebSite',
			'@id': `${canonicalUrl}#website`,
			url: canonicalUrl,
			name: 'Axolotl Launcher',
			description: description.value,
			inLanguage: locale.value,
			publisher: { '@id': `${canonicalUrl}#organization` },
		},
		{
			'@type': 'Organization',
			'@id': `${canonicalUrl}#organization`,
			name: 'Axolotl Launcher Team',
			url: canonicalUrl,
			logo: {
				'@type': 'ImageObject',
				url: `${siteUrl}/axolotl.png`,
				width: 256,
				height: 256,
			},
			sameAs: [githubUrl],
		},
		{
			'@type': 'SoftwareApplication',
			'@id': `${canonicalUrl}#software`,
			name: 'Axolotl Launcher',
			alternateName: ['美西螈启动器', 'AXL Launcher'],
			sameAs: [githubUrl, 'https://cnb.cool/axlmc/Axolotl'],
			description: description.value,
			url: canonicalUrl,
			downloadUrl: `${canonicalUrl}#download`,
			image: socialImageUrl,
			applicationCategory: 'GameApplication',
			applicationSubCategory: 'Minecraft Launcher',
			operatingSystem: 'Windows 10/11, macOS, Linux',
			isAccessibleForFree: true,
			license: licenseUrl,
			softwareHelp: `${githubUrl}#readme`,
			author: { '@id': `${canonicalUrl}#organization` },
			inLanguage: ['zh-CN', 'en-US'],
			featureList: [
				formatMessage(messages.structuredFeatureContentSources),
				formatMessage(messages.structuredFeatureLab),
				formatMessage(messages.structuredFeatureInstances),
				formatMessage(messages.structuredFeatureAccounts),
			],
		},
		{
			'@type': 'FAQPage',
			'@id': `${canonicalUrl}#faq`,
			inLanguage: locale.value,
			mainEntity: faqItems.value.map((item) => ({
				'@type': 'Question',
				name: item.question,
				acceptedAnswer: {
					'@type': 'Answer',
					text: item.answer,
				},
			})),
		},
	],
}))

useSeoMeta({
	title: () => title.value,
	description: () => description.value,
	robots: 'index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1',
	author: 'Axolotl Launcher Team',
	applicationName: 'Axolotl Launcher',
	themeColor: '#ff82b2',
	colorScheme: 'dark light',
	ogTitle: () => title.value,
	ogDescription: () => description.value,
	ogType: 'website',
	ogUrl: canonicalUrl,
	ogSiteName: 'Axolotl Launcher',
	ogLocale: () => locale.value.replace('-', '_'),
	ogLocaleAlternate: () => (locale.value === 'zh-CN' ? 'en_US' : 'zh_CN'),
	ogImage: socialImageUrl,
	ogImageAlt: () => socialImageAlt.value,
	ogImageWidth: 3104,
	ogImageHeight: 1806,
	twitterCard: 'summary_large_image',
	twitterTitle: () => title.value,
	twitterDescription: () => description.value,
	twitterImage: socialImageUrl,
	twitterImageAlt: () => socialImageAlt.value,
})

useHead(() => ({
	link: [{ rel: 'canonical', href: canonicalUrl }],
	meta: [
		{
			name: 'keywords',
			content: keywords.value,
		},
	],
	script: [
		{
			key: 'axolotl-structured-data',
			type: 'application/ld+json',
			innerHTML: JSON.stringify(structuredData.value).replace(/</g, '\\u003c'),
		},
	],
}))
</script>

<template>
	<div>
		<div
			ref="hero"
			class="landing-hero"
			@pointerleave="resetHeroTilt"
			@pointermove="updateHeroTilt"
		>
			<div class="hero-grid" aria-hidden="true" />
			<div class="hero-sun" aria-hidden="true" />
			<div class="hero-wordmark" aria-hidden="true">AXOLOTL</div>
			<div class="hero-content">
				<div class="hero-meta">
					<span class="hero-index">AXL / 01</span>
					<div class="hero-kicker">
						{{ formatMessage(messages.openSourceBadge) }}
					</div>
				</div>
				<h1 class="main-header">{{ formatMessage(messages.downloadAxolotl) }}</h1>
				<p class="main-subheader">
					{{ formatMessage(messages.description) }}
				</p>
				<div class="button-group">
					<ButtonStyled v-if="os" color="brand" size="large">
						<button rel="noopener nofollow" @click="handleDownload">
							<LinuxIcon v-if="os === 'Linux'" />
							<WindowsIcon v-else-if="os === 'Windows'" />
							<AppleIcon v-else-if="os === 'Mac'" />
							{{ formatMessage(messages.downloadAxolotlButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled type="outlined" size="large">
						<button @click="scrollToSection">
							<ArrowDownIcon />
							{{ formatMessage(messages.moreDownloadOptions) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
			<div class="hero-product">
				<div class="hero-product-bar" aria-hidden="true">
					<span />
					<span />
					<span />
				</div>
				<img
					class="hero-screenshot"
					src="/showcase/launcher-home.png"
					:alt="formatMessage(messages.heroScreenshotAlt)"
					width="3104"
					height="1806"
					decoding="async"
					fetchpriority="high"
				/>
			</div>
			<div class="hero-scroll-mark" aria-hidden="true"><span /></div>
			<div class="bottom-transition" />
		</div>
		<section class="axolotl-highlights" aria-labelledby="axolotl-highlights-title">
			<div class="highlights-intro">
				<span class="section-eyebrow">{{ formatMessage(messages.builtOnModrinth) }}</span>
				<h2 id="axolotl-highlights-title">
					{{ formatMessage(messages.highlightsTitle) }}<br />
					{{ formatMessage(messages.highlightsTitleSecond) }}
				</h2>
				<p>{{ formatMessage(messages.highlightsDescription) }}</p>
			</div>

			<div class="modrinth-feature-grid">
				<article class="feature gradient-border promise-card">
					<div class="promise-meta"><CompassIcon /><span>01</span></div>
					<h3>{{ formatMessage(messages.modrinthNative) }}</h3>
					<p>{{ formatMessage(messages.modrinthNativeDescription) }}</p>
				</article>
				<article class="feature gradient-border promise-card">
					<div class="promise-meta"><EyeOffIcon /><span>02</span></div>
					<h3>{{ formatMessage(messages.adFree) }}</h3>
					<p>{{ formatMessage(messages.adFreeDescription) }}</p>
				</article>
				<article class="feature gradient-border promise-card">
					<div class="promise-meta"><LanguagesIcon /><span>03</span></div>
					<h3>{{ formatMessage(messages.localized) }}</h3>
					<p>{{ formatMessage(messages.localizedDescription) }}</p>
				</article>
				<article class="feature gradient-border showcase-card showcase-card-wide">
					<div class="showcase-copy">
						<span>{{ formatMessage(messages.offlineLabel) }}</span>
						<h3>{{ formatMessage(messages.offlineAccounts) }}</h3>
						<p>{{ formatMessage(messages.offlineAccountsDescription) }}</p>
					</div>
					<img
						class="showcase-image"
						src="/showcase/account-login.png"
						:alt="formatMessage(messages.offlineScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>

				<article class="feature gradient-border showcase-card">
					<div class="showcase-copy">
						<span>{{ formatMessage(messages.personalizeLabel) }}</span>
						<h3>{{ formatMessage(messages.themes) }}</h3>
						<p>{{ formatMessage(messages.themesDescription) }}</p>
					</div>
					<img
						class="showcase-image"
						src="/showcase/theme-accent.png"
						:alt="formatMessage(messages.themesScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>

				<article class="feature gradient-border showcase-card">
					<div class="showcase-copy">
						<span>{{ formatMessage(messages.translateLabel) }}</span>
						<h3>{{ formatMessage(messages.translation) }}</h3>
						<p>{{ formatMessage(messages.translationDescription) }}</p>
					</div>
					<img
						class="showcase-image"
						src="/showcase/axolotl-lab.png"
						:alt="formatMessage(messages.translationScreenshotAlt)"
						width="3104"
						height="1814"
						decoding="async"
						loading="lazy"
					/>
				</article>
			</div>
		</section>
		<div id="features" class="features">
			<div class="feature-grid">
				<div class="feature gradient-border mods">
					<div class="search-bar">
						<h4>{{ formatMessage(messages.installedMods) }}</h4>
						<div class="mini-input">
							<SearchIcon aria-hidden="true" />
							<div class="search">{{ formatMessage(messages.searchMods) }}</div>
						</div>
					</div>
					<div class="header row">
						<div />
						<div class="cell">{{ formatMessage(messages.name) }}</div>
						<div class="cell">{{ formatMessage(messages.version) }}</div>
						<div class="cell">{{ formatMessage(messages.actions) }}</div>
					</div>
					<div class="table">
						<div
							v-for="(mod, index) in modManagementData"
							:key="mod.id"
							:class="['row', { first: index === 0 }]"
						>
							<div class="cell">
								<Avatar size="sm" :src="mod.iconUrl" />
							</div>
							<div class="cell">
								<div class="name">{{ mod.name }}</div>
								<div class="description">
									{{ formatMessage(messages.byAuthor, { author: mod.author }) }}
								</div>
							</div>
							<div class="cell">{{ mod.version }}</div>
							<div class="cell check">
								<Checkbox :model-value="true" tabindex="-1" />
								<ButtonStyled circular type="transparent">
									<button tabindex="-1">
										<TrashIcon />
									</button>
								</ButtonStyled>
							</div>
						</div>
					</div>
					<h3>{{ formatMessage(messages.modManagement) }}</h3>
					<p>
						{{ formatMessage(messages.modManagementDescription) }}
					</p>
				</div>
				<div class="feature gradient-border website">
					<img class="website-logo" src="/axolotl.png" alt="" aria-hidden="true" />
					<div class="projects-showcase">
						<div v-for="(row, index) in rows" :key="index" class="row">
							<div v-for="n in 2" :key="n" class="row__content" :class="{ offset: index % 2 }">
								<div
									v-for="project in row"
									:key="project.id"
									class="project button-animation gradient-border"
									:aria-hidden="project.isVisualDuplicate ? 'true' : undefined"
								>
									<Avatar :src="project.icon_url!" alt="" size="sm" />
									<div class="project-info">
										<span class="title">
											{{ project.title }}
										</span>
										<span class="description">
											{{ project.description }}
										</span>
									</div>
								</div>
							</div>
						</div>
					</div>
					<h3>{{ formatMessage(messages.websiteIntegration) }}</h3>
					<p>
						{{ formatMessage(messages.websiteIntegrationDescription) }}
					</p>
				</div>
			</div>
			<div class="feature-row">
				<div class="point">
					<div class="title">
						<OpenSourceIcon />
						<h3>{{ formatMessage(messages.openSource) }}</h3>
					</div>
					<div class="description">
						<IntlFormatted :message-id="messages.openSourceDescription">
							<template #github-link="{ children }">
								<a href="https://github.com/Mystic-Stars/Axolotl" rel="noopener" target="_blank">
									<component :is="() => children" />
								</a>
							</template>
						</IntlFormatted>
					</div>
				</div>
				<div class="point">
					<div class="title">
						<OfflineModeIcon />
						<h3>{{ formatMessage(messages.offlineMode) }}</h3>
					</div>
					<div class="description">
						{{ formatMessage(messages.offlineModeDescription) }}
					</div>
				</div>
				<div class="point">
					<div class="title">
						<MultiplayerIcon />
						<h3>{{ formatMessage(messages.followProjects) }}</h3>
					</div>
					<div class="description">{{ formatMessage(messages.followProjectsDescription) }}</div>
				</div>
			</div>
		</div>
		<section id="faq" class="faq-section" aria-labelledby="faq-title">
			<div class="faq-intro">
				<span class="section-eyebrow">{{ formatMessage(messages.faqEyebrow) }}</span>
				<h2 id="faq-title">{{ formatMessage(messages.faqTitle) }}</h2>
				<p>{{ formatMessage(messages.faqDescription) }}</p>
			</div>
			<div class="faq-list">
				<details v-for="item in faqItems" :key="item.question" class="faq-item">
					<summary>{{ item.question }}</summary>
					<p>{{ item.answer }}</p>
				</details>
			</div>
		</section>
		<div id="download" ref="downloadSection" class="footer">
			<div class="section-badge">{{ formatMessage(messages.downloadOptions) }}</div>
			<div class="section-subheader">
				<div class="section-subheader-title">
					{{ formatMessage(messages.downloadAxolotlTitle) }}
				</div>
				<div class="section-subheader-description">
					{{ formatMessage(messages.downloadDescription) }}
				</div>
			</div>
			<div class="download-section">
				<div class="download-card">
					<div class="title">
						<WindowsLogo />
						{{ formatMessage(messages.windows) }}
					</div>
					<div class="description">
						<a ref="downloadWindows" :href="windowsLink || undefined" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadInstaller) }}</span>
						</a>
					</div>
				</div>
				<div class="divider" />
				<div class="download-card">
					<div class="title">
						<AppleLogo />
						{{ formatMessage(messages.mac) }}
					</div>
					<div class="description apple">
						<a ref="downloadMac" :href="macLinks.universal || undefined" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadInstaller) }}</span>
						</a>
					</div>
				</div>
				<div class="divider" />
				<div class="download-card">
					<div class="title">
						<LinuxLogo />
						<div class="flex">
							{{ formatMessage(messages.linux) }}<span class="text-sm text-secondary">*</span>
						</div>
					</div>
					<div class="description apple">
						<a :href="linuxLinks.appImage || undefined" download="">
							<DownloadIcon />
							<span>{{ formatMessage(messages.downloadAppImage) }}</span>
						</a>
						<Accordion
							class="mt-2 flex flex-col items-center"
							content-class="flex flex-col items-start gap-2 mt-2 text-sm"
							button-class="text-sm text-secondary bg-transparent p-0 w-fit text-left m-0 active:scale-[0.98] transition-transform"
						>
							<template #title="{ open }">
								{{ formatMessage(open ? messages.hideOtherPackages : messages.showOtherPackages) }}
							</template>
							<span class="grid grid-cols-[auto_1fr] gap-2 text-left text-orange"
								><IssuesIcon class="mt-1" /> {{ formatMessage(messages.notRecommended) }}</span
							>
							<a :href="linuxLinks.deb || undefined" download="" class="text-primary">
								<DownloadIcon />
								<span>{{ formatMessage(messages.downloadTheDEB) }}</span>
							</a>
							<a :href="linuxLinks.rpm || undefined" download="" class="text-primary">
								<DownloadIcon />
								<span>{{ formatMessage(messages.downloadTheRPM) }}</span>
							</a>
						</Accordion>
					</div>
				</div>
			</div>
			<p class="terms">
				<IntlFormatted :message-id="messages.downloadTerms">
					<template #terms-link="{ children }">
						<a
							href="https://github.com/Mystic-Stars/Axolotl/blob/main/LICENSE"
							target="_blank"
							rel="noopener"
						>
							<component :is="() => children" />
						</a>
					</template>
					<template #privacy-link="{ children }">
						<NuxtLink to="/privacy">
							<component :is="() => children" />
						</NuxtLink>
					</template>
				</IntlFormatted>
			</p>
			<p class="max-w-[50rem] text-xs text-secondary">
				*<IntlFormatted :message-id="messages.linuxDisclaimer">
					<template #issues-link="{ children }">
						<a
							class="underline hover:brightness-[--hover-brightness]"
							href="https://github.com/Mystic-Stars/Axolotl/releases/latest"
							target="_blank"
							rel="noopener"
						>
							<component :is="() => children" />
						</a>
					</template>
					<template #prism-link="{ children }">
						<a
							class="underline hover:brightness-[--hover-brightness]"
							href="https://github.com/Mystic-Stars/Axolotl/issues"
							target="_blank"
							rel="noopener"
						>
							<component :is="() => children" />
						</a>
					</template>
				</IntlFormatted>
			</p>
		</div>
	</div>
</template>

<style scoped lang="scss">
.faq-section {
	display: grid;
	grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
	gap: 4rem;
	width: min(76rem, calc(100% - 3rem));
	margin: 0 auto;
	padding: 7rem 0;
}

.faq-intro {
	h2 {
		margin: 0.75rem 0 1rem;
		color: var(--color-contrast);
		font-size: clamp(2rem, 4vw, 3.25rem);
		line-height: 1.08;
	}

	p {
		max-width: 32rem;
		margin: 0;
		color: var(--color-secondary);
		font-size: 1.05rem;
		line-height: 1.7;
	}
}

.faq-list {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.faq-item {
	border: 1px solid var(--color-divider);
	border-radius: 1rem;
	background: var(--surface-2);

	summary {
		padding: 1.15rem 1.25rem;
		color: var(--color-contrast);
		font-weight: 700;
		line-height: 1.4;
		cursor: pointer;
	}

	p {
		margin: 0;
		padding: 0 1.25rem 1.25rem;
		color: var(--color-secondary);
		line-height: 1.7;
	}
}

@media (max-width: 800px) {
	.faq-section {
		grid-template-columns: 1fr;
		gap: 2rem;
		width: calc(100% - 2rem);
		padding: 5rem 0;
	}
}

.landing-hero {
	--pointer-x: 50%;
	--pointer-y: 40%;
	--tilt-x: 0deg;
	--tilt-y: 0deg;
	position: relative;
	display: flex;
	min-height: min(63rem, calc(100svh + 8rem));
	align-items: center;
	flex-direction: column;
	overflow: hidden;
	padding: clamp(10rem, 12vw, 11.5rem) 1.5rem 0;
	margin-top: -5.25rem;
	background:
		radial-gradient(
			circle at var(--pointer-x) var(--pointer-y),
			rgb(255 155 197 / 22%),
			transparent 20rem
		),
		radial-gradient(circle at 12% 46%, rgb(70 190 176 / 11%), transparent 26rem),
		linear-gradient(155deg, #161018 0%, #11121a 52%, #121520 100%);
	isolation: isolate;

	&::before,
	&::after {
		position: absolute;
		z-index: -1;
		content: '';
		pointer-events: none;
	}

	&::before {
		inset: 0;
		background: linear-gradient(90deg, rgb(255 255 255 / 3%) 1px, transparent 1px);
		background-size: min(9vw, 9rem) 100%;
		mask-image: linear-gradient(180deg, black, transparent 72%);
	}

	&::after {
		inset: 9.25rem 7% auto;
		height: 1px;
		background: linear-gradient(90deg, transparent, rgb(255 170 206 / 38%), transparent);
	}
}

@media (max-width: 1023px) {
	.landing-hero::after {
		inset: 8.5rem 7% auto;
	}
}

.hero-grid {
	position: absolute;
	inset: 0;
	z-index: -1;
	background-image: linear-gradient(rgb(255 255 255 / 3%) 1px, transparent 1px);
	background-size: 100% min(9vw, 9rem);
	mask-image: linear-gradient(180deg, black, transparent 70%);
	pointer-events: none;
}

.hero-sun {
	position: absolute;
	top: 4.5rem;
	right: clamp(-8rem, 6vw, 5rem);
	z-index: -1;
	width: clamp(21rem, 38vw, 38rem);
	aspect-ratio: 1;
	border: 1px solid rgb(255 185 213 / 14%);
	border-radius: 50%;
	box-shadow:
		0 0 0 5rem rgb(255 160 201 / 2%),
		0 0 0 10rem rgb(255 160 201 / 1%);
	pointer-events: none;
}

.hero-wordmark {
	position: absolute;
	top: clamp(11rem, 20vw, 15rem);
	left: 50%;
	z-index: -1;
	color: rgb(255 255 255 / 3%);
	font-size: clamp(7rem, 21vw, 22rem);
	font-weight: 800;
	letter-spacing: 0;
	line-height: 0.8;
	white-space: nowrap;
	transform: translateX(-50%);
	user-select: none;
}

.hero-content {
	display: flex;
	align-items: center;
	flex-direction: column;
	width: min(100%, 59rem);
	text-align: center;
}

.hero-meta {
	display: flex;
	align-items: center;
	gap: 0.75rem;
}

.hero-index,
.hero-kicker {
	display: inline-flex;
	align-items: center;
	min-height: 2rem;
	padding: 0.35rem 0.65rem;
	border: 1px solid rgb(255 255 255 / 12%);
	font-size: 0.7rem;
	font-weight: 800;
	letter-spacing: 0.08em;
	line-height: 1;
	text-transform: uppercase;
}

.hero-index {
	color: rgb(255 255 255 / 45%);
	font-variant-numeric: tabular-nums;
}

.hero-kicker {
	border-color: color-mix(in srgb, var(--color-brand) 35%, transparent);
	background: color-mix(in srgb, var(--color-brand) 11%, transparent);
	color: var(--color-brand);
	box-shadow: 0 0.75rem 2.5rem color-mix(in srgb, var(--color-brand) 10%, transparent);
}

.main-header {
	max-width: 52rem;
	margin: 1rem 0 2rem;
	color: var(--color-contrast);
	font-size: 5.25rem;
	font-weight: 600;
	letter-spacing: 0;
	line-height: 100%;
	text-wrap: balance;
}

.landing-hero .main-subheader {
	max-width: 46rem;
	margin: 0;
	color: var(--landing-color-subheading);
	font-size: clamp(1rem, 1.6vw, 1.25rem);
	font-weight: 450;
	line-height: 1.65;
	text-wrap: balance;
}

.landing-hero .button-group {
	display: flex;
	flex-wrap: wrap;
	justify-content: center;
	gap: 0.75rem;
	margin: 2rem 0 0;
	mask-image: none;
}

.hero-product {
	position: relative;
	width: min(79rem, 112%);
	margin-top: clamp(3.25rem, 7vw, 5.5rem);
	padding: 0.5rem;
	border: 1px solid rgb(255 255 255 / 16%);
	border-radius: 0.75rem 0.75rem 0 0;
	background: linear-gradient(145deg, rgb(255 255 255 / 15%), rgb(255 255 255 / 2%));
	box-shadow:
		0 2rem 6rem rgb(0 0 0 / 42%),
		0 0 6rem rgb(255 112 172 / 14%);
	transform: perspective(1500px) rotateX(var(--tilt-x)) rotateY(var(--tilt-y));
	transform-origin: center bottom;
	transition: transform 260ms ease-out;
	will-change: transform;
}

.hero-product::after {
	position: absolute;
	inset: 0;
	border: 1px solid rgb(255 255 255 / 6%);
	border-radius: inherit;
	content: '';
	pointer-events: none;
}

.hero-product-bar {
	display: flex;
	gap: 0.33rem;
	padding: 0.2rem 0.3rem 0.7rem;

	span {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 50%;
		background: rgb(255 255 255 / 28%);
	}

	span:first-child {
		background: var(--color-brand);
	}
}

.hero-screenshot {
	display: block;
	width: 100%;
	height: auto;
	border-radius: 0.25rem;
	box-shadow: 0 1px 0 rgb(255 255 255 / 10%) inset;
}

.hero-scroll-mark {
	position: absolute;
	bottom: 2.25rem;
	left: 50%;
	display: flex;
	width: 1px;
	height: 3rem;
	justify-content: flex-start;
	overflow: hidden;
	background: rgb(255 255 255 / 14%);
	transform: translateX(-50%);

	span {
		width: 100%;
		height: 45%;
		background: var(--color-brand);
		animation: scroll-mark 2.3s ease-in-out infinite;
	}
}

@keyframes scroll-mark {
	0%,
	100% {
		transform: translateY(-110%);
	}
	55% {
		transform: translateY(220%);
	}
}

.axolotl-highlights {
	position: relative;
	padding: clamp(5rem, 10vw, 9rem) 1.5rem 3rem;
	background: var(--landing-transition-gradient-end);

	&::before {
		position: absolute;
		top: 0;
		left: 50%;
		width: min(76rem, calc(100% - 3rem));
		height: 1px;
		background: linear-gradient(90deg, transparent, var(--landing-border-color), transparent);
		content: '';
		transform: translateX(-50%);
	}
}

.highlights-intro {
	max-width: 52rem;
	margin: 0 auto clamp(2.75rem, 6vw, 5rem);
	text-align: center;

	h2 {
		margin: 0.7rem 0 1rem;
		color: var(--color-contrast);
		font-size: clamp(2.5rem, 5.5vw, 4.75rem);
		font-weight: 700;
		letter-spacing: 0;
		line-height: 1.04;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 1.05rem;
		line-height: 1.65;
	}
}

.section-eyebrow {
	color: var(--color-brand);
	font-size: 0.75rem;
	font-weight: 800;
	letter-spacing: 0.1em;
	text-transform: uppercase;
}

.modrinth-feature-grid {
	width: min(100%, 68.5rem);
	margin: 0 auto;
	display: grid;
	grid-template-columns: repeat(6, minmax(0, 1fr));
	gap: 1rem;
}

.modrinth-feature-grid .feature {
	padding: var(--gap-xl);
	z-index: 1;
	background: radial-gradient(
		50% 50% at 50% 50%,
		rgba(44, 48, 79, 0.35) 0%,
		rgba(32, 35, 50, 0.27) 100%
	);
	box-shadow:
		0 1.25rem 3rem rgb(0 0 0 / 12%),
		0 0 4rem rgb(57 61 94 / 20%) inset;
	backdrop-filter: blur(6px);
	-webkit-backdrop-filter: blur(6px);
	overflow: hidden;
}

.promise-card {
	grid-column: span 2;
	min-height: 12.5rem;
	padding: 1.5rem;

	.promise-meta {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: var(--color-brand);

		svg {
			width: 1.35rem;
			height: 1.35rem;
		}

		span {
			color: var(--color-brand);
			font-size: 0.75rem;
			font-weight: 800;
			letter-spacing: 0.09em;
		}
	}

	h3 {
		margin: 2.2rem 0 0.65rem;
		color: var(--color-contrast);
		font-size: 1.2rem;
		letter-spacing: -0.025em;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.9rem;
		line-height: 1.6;
	}

	&::after {
		position: absolute;
		right: -1.75rem;
		bottom: -2.25rem;
		color: rgb(255 255 255 / 4%);
		content: attr(data-number);
		font-size: 8rem;
		font-weight: 800;
		line-height: 1;
	}
}

.showcase-card {
	display: flex;
	grid-column: span 3;
	min-width: 0;
	flex-direction: column;
	overflow: hidden;
}

.showcase-card-wide {
	grid-column: 1 / -1;
	display: grid;
	grid-template-columns: minmax(17rem, 0.78fr) minmax(0, 1.22fr);
	align-items: center;
}

.showcase-copy {
	padding: 1.75rem 1.75rem 1.5rem;

	span {
		color: var(--color-brand);
		font-size: 0.72rem;
		font-weight: 800;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	h3 {
		margin: 0.55rem 0 0.65rem;
		color: var(--color-contrast);
		font-size: clamp(1.35rem, 2.4vw, 1.8rem);
		letter-spacing: -0.035em;
		line-height: 1.1;
	}

	p {
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.9rem;
		line-height: 1.6;
	}
}

.showcase-image {
	display: block;
	width: 100%;
	height: auto;
}

:global(html.light-mode) .axolotl-highlights {
	background: #f8f7f8;
}

.features {
	position: relative;
	width: 100%;
	background: var(--landing-transition-gradient-end);
	align-content: center;
	justify-content: center;
	display: flex;
	flex-direction: column;
	padding: 3rem 0 clamp(5rem, 9vw, 8rem);

	h3 {
		font-weight: 500;
		font-size: var(--font-size-xl) !important;
	}

	p {
		font-size: var(--font-size-md) !important;
	}

	.feature-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		grid-template-rows: minmax(33rem, auto);
		max-width: 68.5rem;
		width: min(calc(100% - 3rem), 68.5rem);
		gap: 1rem;
		margin: 0 auto;
		padding: 0;

		.mods {
			grid-column: 1 / 2;
			grid-row: 1 / 2;

			.table {
				margin-bottom: 1rem;
				overflow: hidden;
				max-height: 32rem;
			}

			h3,
			p {
				text-align: center;
			}

			h4 {
				margin: 0;
				color: var(--color-contrast);
			}

			.search-bar {
				width: 100%;
				padding: var(--gap-sm);
				display: flex;
				flex-direction: row;
				justify-content: space-between;
				align-items: center;
				border-radius: var(--radius-md);
				border: 1px solid var(--landing-border-color);
				background: linear-gradient(0deg, #3b3f55 0%, #3b3f55 100%), rgba(59, 63, 85, 0.15);
				margin-bottom: 0.5rem;
				white-space: nowrap;
				font-size: var(--font-size-sm);

				.mini-input {
					display: flex;
					flex-direction: row;
					align-items: center;
					gap: 0.5rem;
					padding: var(--gap-sm) var(--gap-md);
					border-radius: var(--radius-sm);
					background-color: #1e202f;
					flex-grow: 1;
					max-width: 12rem;
				}

				h4 {
					font-weight: normal;
					margin-left: 0.5rem;
				}
			}

			.row {
				display: grid;
				grid-template-columns: 3rem 2fr 1fr 3.75rem;
				padding: 0 var(--gap-sm);
				gap: 1rem;

				.cell {
					display: flex;
					flex-direction: column;
					justify-content: center;
					padding: var(--gap-sm) 0;
					font-size: var(--font-size-sm);

					.name {
						color: var(--color-contrast);
					}

					.description {
						font-size: var(--font-size-xs);
					}

					&.last {
						align-items: flex-end;
					}

					&.check {
						align-items: center;
						flex-direction: row;
					}
				}
			}

			.header {
				.cell {
					color: var(--color-base);
				}
			}
		}

		.website {
			grid-column: 2 / 3;
			grid-row: 1 / 2;
			text-align: center;
			padding: 0 !important;

			position: relative;

			.projects-showcase {
				margin: calc(5rem + var(--gap-xl)) 0 var(--gap-xl);
				z-index: 3;
				text-align: left;

				.row {
					--gap: var(--gap-md);

					width: 100vw;
					gap: var(--gap);
					margin-bottom: var(--gap);
					display: flex;
					overflow: hidden;
					user-select: none;

					.row__content {
						flex-shrink: 0;
						display: flex;
						min-width: 100%;
						gap: var(--gap);
						transform: translateX(-15%);

						&.offset {
							transform: translateX(-130%);
						}
					}

					.project {
						position: relative;
						display: flex;

						cursor: pointer;
						padding: 1rem;
						gap: 1rem;
						border-radius: 1rem;
						border: 1px solid var(--landing-border-color);
						transition:
							background 0.5s ease-in-out,
							transform 0.05s ease-in-out;
						// Removed due to lag on mobile :(
						background: var(--landing-blob-gradient);

						img {
							height: 3rem;
						}

						.project-info {
							box-sizing: border-box;
						}

						.title {
							color: var(--landing-color-heading);
							max-width: 13.75rem;
							overflow: hidden;
							white-space: nowrap;
							text-overflow: ellipsis;
							margin: 0;
							font-weight: 600;
							font-size: 1.25rem;
							line-height: 110%;
							display: block;
						}

						.description {
							width: 13.75rem;

							display: -webkit-box;
							-webkit-line-clamp: 2;
							-webkit-box-orient: vertical;
							overflow: hidden;

							font-weight: 500;
							font-size: 0.875rem;
							line-height: 125%;
							margin: 0.25rem 0 0;
						}
					}
				}
			}

			.website-logo {
				position: absolute;
				top: 1.5rem;
				left: 50%;
				width: 3.5rem;
				height: 3.5rem;
				transform: translateX(-50%);
				z-index: 4;
				object-fit: contain;
			}

			p {
				padding: var(--gap-xl);
				padding-top: 0;
			}
		}

		.feature {
			padding: var(--gap-xl);
			z-index: 1;
			background: radial-gradient(
				50% 50% at 50% 50%,
				rgba(44, 48, 79, 0.35) 0%,
				rgba(32, 35, 50, 0.27) 100%
			);
			box-shadow:
				2px 2px 12px 0 rgba(0, 0, 0, 0.16),
				2px 2px 64px 0 rgba(57, 61, 94, 0.45) inset;
			backdrop-filter: blur(6px);
			-webkit-backdrop-filter: blur(6px);
			max-width: 540px;
			width: 100%;
			overflow: hidden;

			h3,
			p {
				margin: 0;
			}

			h3 {
				font-size: var(--font-size-xl);
				color: var(--landing-color-heading);
				margin-bottom: 0.375rem;
			}

			p {
				color: var(--landing-color-subheading);
			}
		}
	}

	.feature-row {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--gap-lg);
		max-width: 1096px;
		margin: 0 auto;
		padding: calc(var(--gap-xl) * 2) 1rem;

		@media (max-width: 1024px) {
			grid-template-columns: repeat(1, 1fr);

			.point {
				text-align: center;

				.title {
					justify-content: center;
				}
			}
		}

		.point {
			display: flex;
			flex-direction: column;
			gap: var(--gap-md);
			padding: 1rem 0;

			.title {
				display: flex;
				align-items: center;
				gap: 0.5rem;
			}

			h3 {
				font-size: var(--font-size-lg);
				font-weight: normal;
				color: var(--landing-color-heading);
				margin: 0;
			}

			p {
				color: var(--landing-color-subheading);
				margin: 0;
			}

			a {
				text-decoration: underline;
			}
		}
	}
}

.table {
	display: grid;
	border: 1px solid rgba(#a8b1ddbf, 0.25);
	gap: 0.25rem;
	overflow: hidden;
	font-size: var(--font-size-sm);
	background: rgba(59, 63, 85, 0.15);
	box-shadow: 2px 2px 12px 0px rgba(0, 0, 0, 0.16);

	button {
		&:hover {
			cursor: default !important;
		}
	}

	.first {
		border-top: none !important;
	}

	.row {
		&:not(.header) {
			border-top: 1px solid rgba(#a8b1ddbf, 0.25);
		}
	}
}

.row,
.header,
.table,
.project,
.export-card {
	user-select: none;

	&:hover {
		cursor: default;
	}
}

.footer {
	position: relative;
	overflow: hidden;
	padding: clamp(4rem, 8vw, 7rem) var(--gap-xl);
	background: var(--color-accent-contrast);
	color: var(--color-contrast);
	text-align: center;
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
	justify-content: center;
	align-items: center;

	&::before {
		position: absolute;
		top: 0;
		left: 50%;
		width: min(50rem, 90%);
		height: 1px;
		background: linear-gradient(90deg, transparent, var(--color-brand), transparent);
		content: '';
		transform: translateX(-50%);
	}

	.section-badge {
		border: 1px solid color-mix(in srgb, var(--color-brand) 40%, transparent);
		background-color: var(--color-brand-highlight);
		color: var(--color-brand);
		border-radius: 0;
		width: min-content;
		padding: var(--gap-lg) var(--gap-xl);
		white-space: nowrap;
	}

	.section-subheader {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--gap-sm);
		font-size: clamp(1.05rem, 1.5vw, 1.25rem);
		margin: 2rem 0;

		.section-subheader-title {
			font-size: clamp(2rem, 4vw, 3.75rem);
			font-weight: 700;
			letter-spacing: 0;
			line-height: 1;
			margin: 0;
		}

		.section-subheader-description {
			color: var(--color-base);
			margin: 0;
		}
	}

	.download-section {
		display: grid;
		grid-template-columns: 1fr 1px 1fr 1px 1fr;
		height: 100%;
		gap: var(--gap-lg);
		max-width: 1096px;
		margin: 0 auto;

		@media (max-width: 1024px) {
			grid-template-columns: repeat(1, 1fr);
			max-width: 340px;

			.divider {
				display: none;
			}
		}

		.divider {
			height: 13rem;
			width: 1px;
			background: var(--landing-border-color);
			margin: 0;
		}

		.download-card {
			display: flex;
			flex-direction: column;
			gap: calc(var(--gap-lg) * 2);
			padding: calc(var(--gap-lg) * 2);
			height: min-content;

			.title {
				display: flex;
				flex-direction: column;
				align-items: center;
				justify-content: center;
				font-size: var(--font-size-2xl);
				gap: var(--gap-sm);
				border-radius: var(--radius-md) var(--radius-md) 0 0;
				color: var(--color-contrast);
			}

			.description {
				display: flex;
				flex-direction: column;
				align-items: center;
				border-top: none;
				font-size: var(--font-size-md);
				color: var(--color-brand);
				gap: var(--gap-sm);

				a {
					display: flex;
					align-items: center;
					gap: var(--gap-sm);
					justify-content: center;

					&:hover {
						cursor: pointer;
					}

					span {
						text-align: left;
					}
				}
			}

			:deep(.animated-dropdown) {
				color: var(--color-brand);
				width: 16rem;
				white-space: nowrap;

				.selected {
					border: 1px solid var(--color-brand);
					background-color: var(--color-accent-contrast);
				}

				.options {
					border: 1px solid var(--color-brand);
					border-radius: 0 0 var(--radius-md) var(--radius-md);
				}

				.option {
					background-color: var(--color-accent-contrast);
				}

				.selected-option {
					background-color: var(--color-brand);
				}
			}
		}
	}

	.terms {
		margin: var(--gap-xl);
		font-size: var(--font-size-lg);
		color: var(--landing-color-subheading);
		text-align: center;
		line-height: 1.5;

		a {
			text-decoration: underline;
		}
	}
}

.gradient-border {
	position: relative;
	border-radius: var(--radius-lg);

	&:before {
		content: '';
		position: absolute;
		inset: 0;
		padding: 1px;
		z-index: -1;
		border-radius: 1rem;
		background: var(--landing-border-gradient);

		-webkit-mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		-webkit-mask-composite: xor;
		mask-composite: exclude;
	}
}

.bottom-transition {
	position: absolute;
	bottom: 0;
	width: 100%;
	height: 30rem;
	background: linear-gradient(
		0deg,
		var(--landing-transition-gradient-end) 0%,
		var(--landing-transition-gradient-start) 100%
	);
}

@media screen and (max-width: 1024px) {
	.feature-grid {
		grid-template-columns: 1fr !important;
		grid-template-rows: none !important;
		gap: var(--gap-lg);
		margin: 0 auto;
		align-content: center;

		.feature {
			width: 100% !important;
			margin: 0 auto;
		}

		.mods,
		.website {
			grid-row: auto !important;
			grid-column: 1 / 2 !important;
		}
	}

	.main-header {
		font-size: 4rem !important;
	}

	.main-subheader {
		font-size: 1.25rem !important;
	}
}

@media screen and (max-width: 746px) {
	.axolotl-highlights {
		padding: 0 1rem 1rem;
	}

	.highlights-intro {
		margin-bottom: 2rem;

		p {
			font-size: 0.95rem;
		}
	}

	.modrinth-feature-grid {
		grid-template-columns: 1fr;
	}

	.features .feature-grid {
		width: min(calc(100% - 2rem), 68.5rem);
		gap: 1rem;
	}

	.promise-card,
	.showcase-card {
		grid-column: auto;
	}

	.promise-card {
		min-height: auto;
		padding: 1.25rem;

		h3 {
			margin-top: 1.5rem;
		}
	}

	.showcase-card-wide {
		grid-column: auto;
		grid-template-columns: 1fr;
	}

	.showcase-copy {
		padding: 1.35rem 1.25rem 1.15rem;
	}

	.main-header {
		font-size: 3rem !important;
	}

	.main-subheader {
		font-size: 1.1rem !important;
	}
}

.light-mode {
	.footer,
	.features {
		background: #f8f7f8;
	}

	.bottom-transition {
		background: linear-gradient(rgba(#f8f7f8, 0) 0%, #f8f7f8 100%);
	}

	.feature {
		background: radial-gradient(
			50% 50% at 50% 50%,
			rgba(255, 255, 255, 0.35) 0%,
			rgba(255, 255, 255, 0.27) 100%
		) !important;
		box-shadow:
			2px 2px 64px 0px rgba(255, 255, 255, 0.45) inset,
			2px 2px 12px 0px rgba(0, 0, 0, 0.16) !important;
		border: none !important;
	}

	.gradient-border {
		&:before {
			background: var(--landing-border-gradient-light);
		}
	}

	.search-bar {
		background: var(--color-raised-bg) !important;
		border: 2px solid var(--color-brand) !important;

		.mini-input {
			background: var(--color-raised-bg) !important;
			border: 2px solid var(--color-bg);
		}
	}

	.landing-hero {
		background:
			radial-gradient(circle at 18% 20%, rgb(239 126 170 / 20%), transparent 28rem),
			radial-gradient(circle at 82% 36%, rgb(142 119 230 / 11%), transparent 32rem),
			linear-gradient(180deg, #fff9fc 0%, #faf6fa 58%, #f8f4f7 100%);

		.hero-wordmark {
			color: rgb(105 73 88 / 8%);
		}
	}

	.table {
		background: white;
	}

	.project {
		background: rgba(255, 255, 255, 0.8) !important;
	}

	.export-card {
		background: white !important;
	}
}
</style>
