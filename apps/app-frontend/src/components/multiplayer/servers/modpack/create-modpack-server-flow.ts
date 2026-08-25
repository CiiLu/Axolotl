import type { Labrinth } from '@modrinth/api-client'
import { RefreshCwIcon } from '@modrinth/assets'
import { type ServerTypeId, setEulaAccepted } from '@modrinth/server'
import {
	createContext,
	defineMessages,
	type MultiStageModal,
	type StageConfigInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, markRaw, type Ref, ref } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import { refresh as refreshServerList } from '@/composables/useServers'
import { find_filtered_jres, get_java_default_versions, get_max_memory } from '@/helpers/jre'
import { get_loader_versions } from '@/helpers/metadata'
import { serverEventListener, type ServerManifestData, servers } from '@/helpers/servers'

import type { CreateServerFlowContext, JavaSelection } from '../create-server-flow'
import {
	javaMajorFromVersion,
	resolveServerLauncher,
	toErrorMessage,
	waitForServerStop,
} from '../server-flow-utils'
import ConfigureStage from '../stages/ConfigureStage.vue'
import ModpackInstallStage from './stages/ModpackInstallStage.vue'
import ModpackSetupStage from './stages/ModpackSetupStage.vue'

export type ModpackInstallPhase =
	| 'idle'
	| 'preparing'
	| 'downloading'
	| 'first-run'
	| 'eula'
	| 'error'
	| 'done'

export interface ModpackServerOptions {
	project: Labrinth.Projects.v2.Project
	version: Labrinth.Versions.v2.Version
}

export interface ModpackServerFlowContext extends CreateServerFlowContext {
	modpackTitle: Ref<string>
	modpackVersionNumber: Ref<string>
	modpackIconUrl: Ref<string | undefined>
	loaderLabel: Ref<string>
	loaderSupported: Ref<boolean>
	gameVersionLabel: Ref<string>
	setPack: (project: Labrinth.Projects.v2.Project, version: Labrinth.Versions.v2.Version) => void
}

export const [injectModpackServerFlow, provideModpackServerFlow] =
	createContext<ModpackServerFlowContext>('ModpackServerFlow')

const MODPACK_SERVER_TYPES: Record<string, { type: ServerTypeId; label: string }> = {
	fabric: { type: 'fabric', label: 'Fabric' },
	quilt: { type: 'quilt', label: 'Quilt' },
	neoforge: { type: 'neoforge', label: 'NeoForge' },
	forge: { type: 'forge', label: 'Forge' },
}

/** Loaders whose server launcher the app can download and boot directly. */
const SUPPORTED_MODPACK_LOADERS: ServerTypeId[] = ['vanilla', 'fabric', 'quilt']

export function resolveModpackLoader(loaders: string[]): { type: ServerTypeId; label: string } {
	for (const loader of loaders) {
		const entry = MODPACK_SERVER_TYPES[loader.toLowerCase()]
		if (entry) return entry
	}
	return { type: 'vanilla', label: 'Vanilla' }
}

export function createModpackServerFlowContext(
	modal: Ref<ComponentExposed<typeof MultiStageModal> | null>,
): ModpackServerFlowContext {
	const { formatMessage } = useVIntl()

	const wizardMessages = defineMessages({
		setupTitle: { id: 'app.servers.wizard.setup-title', defaultMessage: 'Setup' },
		installTitle: { id: 'app.servers.wizard.install-title', defaultMessage: 'Install' },
		configureTitle: { id: 'app.servers.wizard.configure-title', defaultMessage: 'Configure' },
		next: { id: 'app.servers.wizard.next', defaultMessage: 'Next' },
		retry: { id: 'app.servers.wizard.retry', defaultMessage: 'Retry' },
		finish: { id: 'app.servers.wizard.finish', defaultMessage: 'Finish' },
		javaTooOld: {
			id: 'app.servers.wizard.java-too-old',
			defaultMessage:
				'Java {selected} cannot run this game version; Java {required} or newer is required.',
		},
		firstRunCrashed: {
			id: 'app.servers.modpack.first-run-crashed',
			defaultMessage:
				'The server crashed during its first start. Check that your selected Java version is compatible, then try again.',
		},
	})

	const project = ref<Labrinth.Projects.v2.Project | null>(null)
	const version = ref<Labrinth.Versions.v2.Version | null>(null)

	const serverType = ref<ServerTypeId>('vanilla')
	const availableGameVersions = ref<string[]>([])
	const selectedGameVersion = ref('')
	const showSnapshots = ref(false)
	const loaderVersions = ref<{ id: string; stable: boolean }[]>([])
	const selectedLoaderVersion = ref('')
	const isVersionsLoading = ref(false)
	const versionsError = ref<string | null>(null)

	const name = ref('')
	const selectedJava = ref<JavaSelection>({ path: '', version: '' })
	const memoryMb = ref(2048)
	const maxMemoryMb = ref(8192)

	const installPhase = ref<ModpackInstallPhase>('idle')
	const downloadProgress = ref<{ downloaded: number; total: number | null } | null>(null)
	const installLog = ref<string[]>([])
	const installError = ref<string | null>(null)
	const eulaText = ref('')
	const createdServer = ref<ServerManifestData | null>(null)
	const showEulaModal = ref(false)
	const saveServerProperties = ref<(() => Promise<boolean>) | null>(null)

	const modpackTitle = ref('')
	const modpackVersionNumber = ref('')
	const modpackIconUrl = ref<string | undefined>(undefined)
	const loaderLabel = ref('')
	const loaderSupported = ref(false)
	const gameVersionLabel = ref('')

	const needsLoaderVersion = computed(
		() => serverType.value === 'fabric' || serverType.value === 'quilt',
	)
	const typeSupported = computed(() => loaderSupported.value)
	const canContinueFromType = computed(() => loaderSupported.value)

	function setPack(
		packProject: Labrinth.Projects.v2.Project,
		packVersion: Labrinth.Versions.v2.Version,
	) {
		project.value = packProject
		version.value = packVersion
		modpackTitle.value = packProject.title
		modpackVersionNumber.value = packVersion.version_number ?? ''
		modpackIconUrl.value = packProject.icon_url ?? undefined

		const gameVersion = packVersion.game_versions?.[0] ?? ''
		const loader = resolveModpackLoader(packVersion.loaders ?? [])
		serverType.value = loader.type
		loaderLabel.value = loader.label
		gameVersionLabel.value = gameVersion
		selectedGameVersion.value = gameVersion
		availableGameVersions.value = gameVersion ? [gameVersion] : []
		loaderSupported.value = SUPPORTED_MODPACK_LOADERS.includes(loader.type)

		name.value = packProject.title
		selectedLoaderVersion.value = ''
		loaderVersions.value = []
	}

	async function loadVersions() {
		// The modpack fixes the game version; nothing to load.
	}

	async function loadLoaderVersions() {
		selectedLoaderVersion.value = ''
		loaderVersions.value = []
		if (!needsLoaderVersion.value || !selectedGameVersion.value) return
		try {
			const manifest = (await get_loader_versions(serverType.value, selectedGameVersion.value)) as {
				gameVersions: Array<{ id: string; loaders: { id: string; stable: boolean }[] }>
			}
			const entry = manifest.gameVersions.find((game) => game.id === selectedGameVersion.value)
			loaderVersions.value = entry?.loaders ?? []
			const stable = loaderVersions.value.find((option) => option.stable) ?? loaderVersions.value[0]
			selectedLoaderVersion.value = stable?.id ?? ''
		} catch {
			loaderVersions.value = []
		}
	}

	async function loadDefaultJava() {
		if (selectedJava.value.path !== '') return
		const major = javaMajorFromVersion(selectedGameVersion.value || '1.21') ?? 21
		try {
			const defaults = (await get_java_default_versions()) as Array<{
				parsed_version: number
				version: string
				path: string
			}>
			const match =
				defaults.find((entry) => entry.parsed_version === major) ??
				defaults.find((entry) => entry.parsed_version >= major)
			if (match) {
				selectedJava.value = { path: match.path, version: match.version }
				return
			}
		} catch {
			// Fall through to a filtered scan
		}
		try {
			const javas = (await find_filtered_jres(major)) as JavaSelection[]
			if (javas.length > 0) selectedJava.value = javas[0]
		} catch {
			// Leave empty; the user picks manually in the setup stage
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

	async function beginInstall() {
		if (installPhase.value === 'downloading' || installPhase.value === 'first-run') return
		if (!project.value || !version.value) return
		if (!loaderSupported.value) return

		installPhase.value = 'preparing'
		installError.value = null
		installLog.value = []
		downloadProgress.value = null
		try {
			await loadLoaderVersions()

			const requiredJava = javaMajorFromVersion(selectedGameVersion.value) ?? 21
			const selectedMajor = javaMajorFromVersion(selectedJava.value.version)
			if (
				selectedJava.value.path !== '' &&
				selectedMajor !== null &&
				selectedMajor < requiredJava
			) {
				throw new Error(
					formatMessage(wizardMessages.javaTooOld, {
						selected: selectedMajor,
						required: requiredJava,
					}),
				)
			}

			const manifest = await servers.create({
				name: name.value,
				serverType: serverType.value,
				gameVersion: selectedGameVersion.value,
				loaderVersion: needsLoaderVersion.value ? selectedLoaderVersion.value : undefined,
				javaPath: selectedJava.value.path || undefined,
				memoryMb: memoryMb.value,
			})
			createdServer.value = manifest

			const jar = await resolveServerLauncher(
				serverType.value,
				selectedGameVersion.value,
				selectedLoaderVersion.value,
			)
			if (!jar) {
				throw new Error(
					`No server launcher available for ${loaderLabel.value} on ${selectedGameVersion.value}`,
				)
			}

			const primaryFile = version.value.files.find((file) => file.primary) ?? version.value.files[0]
			if (!primaryFile?.url) {
				throw new Error('Modpack has no downloadable file')
			}

			installPhase.value = 'downloading'
			const unlistenProgress = await serverEventListener((serverId, payload) => {
				if (serverId !== manifest.id || payload.event !== 'download_progress') return
				downloadProgress.value = {
					downloaded: payload.downloaded,
					total: payload.total ?? null,
				}
			})
			const unlistenLogs = await serverEventListener((serverId, payload) => {
				if (serverId !== manifest.id || payload.event !== 'log') return
				installLog.value.push(payload.line)
				if (installLog.value.length > 500) {
					installLog.value.splice(0, installLog.value.length - 500)
				}
			})
			try {
				await servers.installModpack(manifest.id, {
					mrpackUrl: primaryFile.url,
					mrpackSha1: primaryFile.hashes?.sha1,
					jarUrl: jar.url,
					jarFilename: jar.filename,
					jarSha1: jar.sha1,
					modpackProjectId: project.value.id,
					modpackVersionId: version.value.id,
					modpackTitle: modpackTitle.value,
					modpackIconUrl: modpackIconUrl.value,
				})
			} finally {
				unlistenProgress()
				unlistenLogs()
			}

			installPhase.value = 'first-run'
			const unlistenFirstRunLogs = await serverEventListener((serverId, payload) => {
				if (serverId !== manifest.id || payload.event !== 'log') return
				installLog.value.push(payload.line)
				if (installLog.value.length > 500) {
					installLog.value.splice(0, installLog.value.length - 500)
				}
			})
			try {
				await servers.start(manifest.id)
				const stopped = await waitForServerStop(manifest.id)
				if (stopped?.event === 'stopped' && stopped.crashed) {
					throw new Error(formatMessage(wizardMessages.firstRunCrashed))
				}
			} finally {
				unlistenFirstRunLogs()
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
			installError.value = toErrorMessage(error)
			// A half-installed server must not linger in the list; retrying starts over.
			if (createdServer.value) {
				const failed = createdServer.value
				createdServer.value = null
				await servers.delete(failed.id).catch(() => {})
				void refreshServerList()
			}
		}
	}

	function retryInstall(): Promise<void> {
		installPhase.value = 'idle'
		return beginInstall()
	}

	async function acceptEula() {
		if (!createdServer.value) return
		try {
			const updated = setEulaAccepted(eulaText.value, true)
			await servers.writeFile(createdServer.value.id, 'eula.txt', updated)
			showEulaModal.value = false
			installPhase.value = 'done'
		} catch (error) {
			installError.value = toErrorMessage(error)
			installPhase.value = 'error'
			showEulaModal.value = false
		}
	}

	function declineEula() {
		showEulaModal.value = false
		modal.value?.hide()
	}

	function reset() {
		installPhase.value = 'idle'
		installLog.value = []
		installError.value = null
		downloadProgress.value = null
		eulaText.value = ''
		createdServer.value = null
		showEulaModal.value = false
		saveServerProperties.value = null
		selectedJava.value = { path: '', version: '' }
		memoryMb.value = 2048
		void loadMaxMemory()
	}

	const stageConfigs: StageConfigInput<CreateServerFlowContext>[] = [
		{
			id: 'setup',
			stageContent: markRaw(ModpackSetupStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.setupTitle),
			cannotNavigateForward: (ctx) =>
				ctx.name.value.trim() === '' || !ctx.canContinueFromType.value,
			leftButtonConfig: () => null,
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(wizardMessages.next),
				color: 'brand',
				disabled: ctx.name.value.trim() === '' || !ctx.canContinueFromType.value,
				onClick: async () => {
					await ctx.loadDefaultJava()
					ctx.modal.value?.nextStage()
				},
			}),
		},
		{
			id: 'install',
			stageContent: markRaw(ModpackInstallStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.installTitle),
			cannotNavigateForward: (ctx) => ctx.installPhase.value !== 'done',
			disableClose: (ctx) =>
				ctx.installPhase.value === 'downloading' || ctx.installPhase.value === 'first-run',
			leftButtonConfig: () => null,
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(
					ctx.installPhase.value === 'error' ? wizardMessages.retry : wizardMessages.next,
				),
				color: 'brand',
				icon: ctx.installPhase.value === 'error' ? RefreshCwIcon : null,
				iconPosition: 'after',
				disabled: ctx.installPhase.value !== 'done' && ctx.installPhase.value !== 'error',
				onClick: () => {
					if (ctx.installPhase.value === 'error') {
						void ctx.retryInstall()
						return
					}
					ctx.modal.value?.nextStage()
				},
			}),
		},
		{
			id: 'configure',
			stageContent: markRaw(ConfigureStage),
			title: (ctx) => ctx.formatMessage(wizardMessages.configureTitle),
			maxWidth: 'min(60rem, calc(95vw - 10rem))',
			leftButtonConfig: () => null,
			rightButtonConfig: (ctx) => ({
				label: ctx.formatMessage(wizardMessages.finish),
				color: 'brand',
				onClick: async () => {
					const save = ctx.saveServerProperties.value
					if (save === null || (await save())) ctx.modal.value?.hide()
				},
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
		selectedJava,
		memoryMb,
		maxMemoryMb,
		installPhase,
		downloadProgress,
		installLog,
		installError,
		eulaText,
		createdServer,
		showEulaModal,
		saveServerProperties,
		needsLoaderVersion,
		typeSupported,
		canContinueFromType,
		modpackTitle,
		modpackVersionNumber,
		modpackIconUrl,
		loaderLabel,
		loaderSupported,
		gameVersionLabel,
		loadVersions,
		loadLoaderVersions,
		loadDefaultJava,
		beginInstall,
		retryInstall,
		acceptEula,
		declineEula,
		reset,
		setPack,
	}
}
