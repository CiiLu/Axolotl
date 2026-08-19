import { DownloadIcon, RefreshCwIcon } from '@modrinth/assets'
import {
	fabricInstallerVersionsUrl,
	type FabricLoaderVersionsResponse,
	isServerTypeSupported,
	latestPaperBuild,
	type PaperBuildsResponse,
	requiredJavaMajorVersion,
	resolveServerJar,
	type ServerTypeId,
	setEulaAccepted,
} from '@modrinth/server'
import {
	createContext,
	defineMessages,
	type MultiStageModal,
	type StageConfigInput,
	useVIntl,
} from '@modrinth/ui'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { computed, markRaw, type Ref,ref } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import { auto_install_java, find_filtered_jres, get_max_memory } from '@/helpers/jre'
import { get_game_versions, get_loader_versions } from '@/helpers/metadata'
import {
	serverEventListener,
	type ServerEventPayload,
	type ServerManifestData,
	servers,
} from '@/helpers/servers'

import ConfigureStage from './stages/ConfigureStage.vue'
import InstallStage from './stages/InstallStage.vue'
import SetupStage from './stages/SetupStage.vue'
import TypeStage from './stages/TypeStage.vue'

export type InstallPhase =
	| 'idle'
	| 'preparing'
	| 'downloading'
	| 'first-run'
	| 'eula'
	| 'error'
	| 'done'

export interface JavaOption {
	path: string
	version: string
}

export interface LoaderVersionOption {
	id: string
	stable: boolean
}

export interface CreateServerFlowContext {
	modal: Ref<ComponentExposed<typeof MultiStageModal> | null>
	stageConfigs: StageConfigInput<CreateServerFlowContext>[]
	formatMessage: ReturnType<typeof useVIntl>['formatMessage']

	serverType: Ref<ServerTypeId>
	availableGameVersions: Ref<string[]>
	selectedGameVersion: Ref<string>
	showSnapshots: Ref<boolean>
	loaderVersions: Ref<LoaderVersionOption[]>
	selectedLoaderVersion: Ref<string>
	isVersionsLoading: Ref<boolean>
	versionsError: Ref<string | null>

	name: Ref<string>
	javaOptions: Ref<JavaOption[]>
	selectedJavaPath: Ref<string>
	memoryMb: Ref<number>
	maxMemoryMb: Ref<number>
	isJavaLoading: Ref<boolean>
	isInstallingJava: Ref<boolean>

	installPhase: Ref<InstallPhase>
	downloadProgress: Ref<{ downloaded: number; total: number | null } | null>
	installLog: Ref<string[]>
	installError: Ref<string | null>
	eulaText: Ref<string>
	createdServer: Ref<ServerManifestData | null>
	showEulaModal: Ref<boolean>

	needsLoaderVersion: Ref<boolean>
	typeSupported: Ref<boolean>
	canContinueFromType: Ref<boolean>

	loadVersions: () => Promise<void>
	loadJavaOptions: () => Promise<void>
	installJava: () => Promise<void>
	beginInstall: () => Promise<void>
	retryInstall: () => Promise<void>
	acceptEula: () => Promise<void>
	declineEula: () => void
	reset: () => void
}

export const [injectCreateServerFlow, provideCreateServerFlow] =
	createContext<CreateServerFlowContext>('CreateServerFlow')

interface VanillaVersionEntry {
	id: string
	type: string
	url: string
}

interface VanillaVersionInfoJson {
	downloads?: { server?: { sha1: string; size: number; url: string } }
}

async function fetchJson<T>(url: string): Promise<T> {
	const response = await tauriFetch(url)
	if (!response.ok) throw new Error('GET ' + url + ' failed: ' + response.status)
	return (await response.json()) as T
}

async function waitForServerStop(serverId: string): Promise<ServerEventPayload | null> {
	return new Promise((resolve) => {
		void serverEventListener((eventServerId, payload) => {
			if (eventServerId !== serverId || payload.event !== 'stopped') return
			resolve(payload)
		}).then((unlisten) => {
			setTimeout(() => {
				unlisten()
				resolve(null)
			}, 10 * 60 * 1000)
		})
	})
}

export function createCreateServerFlowContext(
	modal: Ref<ComponentExposed<typeof MultiStageModal> | null>,
): CreateServerFlowContext {
	const { formatMessage } = useVIntl()

	const wizardMessages = defineMessages({
		typeStageTitle: { id: 'app.servers.wizard.type-title', defaultMessage: 'Server type' },
		setupStageTitle: { id: 'app.servers.wizard.setup-title', defaultMessage: 'Setup' },
		installStageTitle: { id: 'app.servers.wizard.install-title', defaultMessage: 'Install' },
		configureStageTitle: { id: 'app.servers.wizard.configure-title', defaultMessage: 'Configure' },
		next: { id: 'app.servers.wizard.next', defaultMessage: 'Next' },
		retry: { id: 'app.servers.wizard.retry', defaultMessage: 'Retry' },
		finish: { id: 'app.servers.wizard.finish', defaultMessage: 'Finish' },
	})

	const serverType = ref<ServerTypeId>('vanilla')
	const availableGameVersions = ref<string[]>([])
	const selectedGameVersion = ref('')
	const showSnapshots = ref(false)
	const loaderVersions = ref<LoaderVersionOption[]>([])
	const selectedLoaderVersion = ref('')
	const isVersionsLoading = ref(false)
	const versionsError = ref<string | null>(null)

	const name = ref('')
	const javaOptions = ref<JavaOption[]>([])
	const selectedJavaPath = ref('')
	const memoryMb = ref(2048)
	const maxMemoryMb = ref(8192)
	const isJavaLoading = ref(false)
	const isInstallingJava = ref(false)

	const installPhase = ref<InstallPhase>('idle')
	const downloadProgress = ref<{ downloaded: number; total: number | null } | null>(null)
	const installLog = ref<string[]>([])
	const installError = ref<string | null>(null)
	const eulaText = ref('')
	const createdServer = ref<ServerManifestData | null>(null)
	const showEulaModal = ref(false)

	const needsLoaderVersion = computed(() => serverType.value === 'fabric')
	const typeSupported = computed(() => isServerTypeSupported(serverType.value))

	async function loadVersions() {
		isVersionsLoading.value = true
		versionsError.value = null
		try {
			const manifest = (await get_game_versions()) as {
				latest: { release: string }
				versions: VanillaVersionEntry[]
			}
			const all = manifest.versions
			availableGameVersions.value = all
				.filter((entry) => (showSnapshots.value ? true : entry.type === 'release'))
				.map((entry) => entry.id)
			if (!availableGameVersions.value.includes(selectedGameVersion.value)) {
				selectedGameVersion.value =
					manifest.latest.release && availableGameVersions.value.includes(manifest.latest.release)
						? manifest.latest.release
						: availableGameVersions.value[0]
			}
			await loadLoaderVersions()
		} catch (error) {
			versionsError.value = error instanceof Error ? error.message : String(error)
		} finally {
			isVersionsLoading.value = false
		}
	}

	async function loadLoaderVersions() {
		selectedLoaderVersion.value = ''
		loaderVersions.value = []
		if (serverType.value !== 'fabric' || !selectedGameVersion.value) return
		try {
			const manifest = (await get_loader_versions('fabric', selectedGameVersion.value)) as {
				gameVersions: Array<{ id: string; loaders: LoaderVersionOption[] }>
			}
			const entry = manifest.gameVersions.find((game) => game.id === selectedGameVersion.value)
			loaderVersions.value = entry?.loaders ?? []
			selectedLoaderVersion.value = loaderVersions.value[0]?.id ?? ''
		} catch {
			loaderVersions.value = []
		}
	}

	async function loadJavaOptions() {
		isJavaLoading.value = true
		try {
			const major = requiredJavaMajorVersion(selectedGameVersion.value || '1.21')
			const javas = (await find_filtered_jres(major)) as JavaOption[]
			javaOptions.value = javas
		} finally {
			isJavaLoading.value = false
		}
	}

	async function loadMaxMemory() {
		try {
			const maxKiB = (await get_max_memory()) as number
			maxMemoryMb.value = Math.max(1024, Math.floor(maxKiB / 1024))
		} catch {
			maxMemoryMb.value = 8192
		}
	}

	async function installJava() {
		if (isInstallingJava.value) return
		isInstallingJava.value = true
		try {
			const major = requiredJavaMajorVersion(selectedGameVersion.value || '1.21')
			await auto_install_java(major)
			await loadJavaOptions()
		} finally {
			isInstallingJava.value = false
		}
	}

	async function beginInstall() {
		if (installPhase.value === 'downloading' || installPhase.value === 'first-run') return
		installPhase.value = 'preparing'
		installError.value = null
		installLog.value = []
		downloadProgress.value = null
		try {
			const manifest = await servers.create({
				name: name.value,
				serverType: serverType.value,
				gameVersion: selectedGameVersion.value,
				loaderVersion: serverType.value === 'fabric' ? selectedLoaderVersion.value : undefined,
				javaPath: selectedJavaPath.value || undefined,
				memoryMb: memoryMb.value,
			})
			createdServer.value = manifest

			let url = ''
			let filename = ''
			let sha1: string | undefined
			if (serverType.value === 'vanilla') {
				const versionManifest = (await get_game_versions()) as {
					versions: VanillaVersionEntry[]
				}
				const entry = versionManifest.versions.find((v) => v.id === selectedGameVersion.value)
				if (!entry) throw new Error('Game version not found in the Mojang manifest')
				const versionInfo = await fetchJson<VanillaVersionInfoJson>(entry.url)
				const jar = resolveServerJar('vanilla', {
					gameVersion: selectedGameVersion.value,
					vanillaVersionInfo: { downloads: versionInfo.downloads },
				})
				if (!jar) throw new Error('This game version has no server download')
				url = jar.url
				filename = jar.filename
				sha1 = jar.sha1
			} else if (serverType.value === 'fabric') {
				const installers = await fetchJson<FabricLoaderVersionsResponse>(
					fabricInstallerVersionsUrl(),
				)
				const installerVersion = installers.loader?.[0]?.version
				const jar = resolveServerJar('fabric', {
					gameVersion: selectedGameVersion.value,
					loaderVersion: selectedLoaderVersion.value,
					installerVersion,
				})
				if (!jar) throw new Error('Fabric server launcher is unavailable for this version')
				url = jar.url
				filename = jar.filename
			} else if (serverType.value === 'paper') {
				const builds = await fetchJson<PaperBuildsResponse>(
					'https://api.papermc.io/v2/projects/paper/versions/' +
						encodeURIComponent(selectedGameVersion.value) +
						'/builds',
				)
				const build = latestPaperBuild(builds)
				const jar = resolveServerJar('paper', {
					gameVersion: selectedGameVersion.value,
					paperBuild: build ?? undefined,
				})
				if (!jar) throw new Error('Paper has no build for this game version')
				url = jar.url
				filename = jar.filename
			}

			installPhase.value = 'downloading'
			const unlistenProgress = await serverEventListener((serverId, payload) => {
				if (serverId !== manifest.id || payload.event !== 'download_progress') return
				downloadProgress.value = {
					downloaded: payload.downloaded,
					total: payload.total ?? null,
				}
			})
			try {
				await servers.downloadFile(manifest.id, url, filename, sha1)
			} finally {
				unlistenProgress()
			}

			installPhase.value = 'first-run'
			const unlistenLogs = await serverEventListener((serverId, payload) => {
				if (serverId !== manifest.id || payload.event !== 'log') return
				installLog.value.push(payload.line)
				if (installLog.value.length > 500) installLog.value.splice(0, installLog.value.length - 500)
			})
			try {
				await servers.start(manifest.id)
				await waitForServerStop(manifest.id)
			} finally {
				unlistenLogs()
			}

			const eula = await servers.readFile(manifest.id, 'eula.txt').catch(() => null)
			if (eula !== null && !eula.includes('eula=true')) {
				eulaText.value = eula
				showEulaModal.value = true
				installPhase.value = 'eula'
				return
			}
			installPhase.value = 'done'
		} catch (error) {
			installPhase.value = 'error'
			installError.value = error instanceof Error ? error.message : String(error)
		}
	}

	function retryInstall() {
		installPhase.value = 'idle'
		void beginInstall()
	}

	async function acceptEula() {
		if (!createdServer.value) return
		try {
			const updated = setEulaAccepted(eulaText.value, true)
			await servers.writeFile(createdServer.value.id, 'eula.txt', updated)
			showEulaModal.value = false
			installPhase.value = 'done'
		} catch (error) {
			installError.value = error instanceof Error ? error.message : String(error)
			installPhase.value = 'error'
			showEulaModal.value = false
		}
	}

	function declineEula() {
		showEulaModal.value = false
		modal.value?.hide()
	}

	function reset() {
		serverType.value = 'vanilla'
		selectedGameVersion.value = ''
		selectedLoaderVersion.value = ''
		loaderVersions.value = []
		name.value = ''
		selectedJavaPath.value = ''
		memoryMb.value = 2048
		installPhase.value = 'idle'
		installLog.value = []
		installError.value = null
		downloadProgress.value = null
		eulaText.value = ''
		createdServer.value = null
		showEulaModal.value = false
		void loadVersions()
		void loadMaxMemory()
	}

	const canContinueFromType = computed(
		() =>
			typeSupported.value &&
			selectedGameVersion.value !== '' &&
			(!needsLoaderVersion.value || selectedLoaderVersion.value !== ''),
	)

	const stageConfigs: StageConfigInput<CreateServerFlowContext>[] = [
		{
			id: 'type',
			stageContent: markRaw(TypeStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.typeStageTitle),
			cannotNavigateForward: (ctx) => !ctx.canContinueFromType.value,
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(wizardMessages.next),
				icon: DownloadIcon,
				iconPosition: 'after',
				disabled: !ctx.canContinueFromType.value,
				onClick: () => ctx.modal.value?.nextStage(),
			}),
		},
		{
			id: 'setup',
			stageContent: markRaw(SetupStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.setupStageTitle),
			cannotNavigateForward: (ctx) => ctx.name.value.trim() === '',
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(wizardMessages.next),
				icon: DownloadIcon,
				iconPosition: 'after',
				disabled: ctx.name.value.trim() === '',
				onClick: async () => {
					await ctx.loadJavaOptions()
					ctx.modal.value?.nextStage()
				},
			}),
		},
		{
			id: 'install',
			stageContent: markRaw(InstallStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.installStageTitle),
			cannotNavigateForward: (ctx) => ctx.installPhase.value !== 'done',
			disableClose: (ctx) =>
				ctx.installPhase.value === 'downloading' || ctx.installPhase.value === 'first-run',
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(
					ctx.installPhase.value === 'error' ? wizardMessages.retry : wizardMessages.next,
				),
				icon: ctx.installPhase.value === 'error' ? RefreshCwIcon : DownloadIcon,
				iconPosition: 'after',
				disabled: ctx.installPhase.value !== 'done' && ctx.installPhase.value !== 'error',
				onClick: () => {
					if (ctx.installPhase.value === 'error') {
						ctx.retryInstall()
						return
					}
					ctx.modal.value?.nextStage()
				},
			}),
		},
		{
			id: 'configure',
			stageContent: markRaw(ConfigureStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.configureStageTitle),
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(wizardMessages.finish),
				onClick: () => ctx.modal.value?.hide(),
			}),
		},
	]

	return {
		modal,
		stageConfigs,
		formatMessage,
		serverType,
		availableGameVersions,
		selectedGameVersion,
		showSnapshots,
		loaderVersions,
		selectedLoaderVersion,
		isVersionsLoading,
		versionsError,
		name,
		javaOptions,
		selectedJavaPath,
		memoryMb,
		maxMemoryMb,
		isJavaLoading,
		isInstallingJava,
		installPhase,
		downloadProgress,
		installLog,
		installError,
		eulaText,
		createdServer,
		showEulaModal,
		needsLoaderVersion,
		typeSupported,
		canContinueFromType,
		loadVersions,
		loadJavaOptions,
		installJava,
		beginInstall,
		retryInstall,
		acceptEula,
		declineEula,
		reset,
	}
}
