import type { Labrinth } from '@modrinth/api-client'
import { reactive } from 'vue'

import { resolveServerLauncher } from '@/components/multiplayer/servers/server-flow-utils'
import { get_version } from '@/helpers/cache.js'
import {
	type InstallModpackOptions,
	serverEventListener,
	type ServerInfoData,
	servers,
} from '@/helpers/servers'

import { refresh as refreshServerList } from './useServers'

const LOG_CAPACITY = 500

export interface ActiveServerInstall {
	progress: { downloaded: number; total: number | null } | null
	log: string[]
}

/**
 * Why a server's start controls are replaced by download actions:
 * - `installing`: files are transferring right now (this app session)
 * - `interrupted`: the manifest was flagged mid-install and never finished
 *   (app closed while downloading); resumable
 * - `failed`: the last install attempt errored; retryable
 */
export type ServerSetupStatus = 'installing' | 'interrupted' | 'failed'

/** Module-level singleton: install activity is global to the app. */
const activeInstalls = reactive<Record<string, ActiveServerInstall | undefined>>({})

export function activeInstallFor(serverId: string): ActiveServerInstall | null {
	return activeInstalls[serverId] ?? null
}

export function serverSetupStatus(server: ServerInfoData): ServerSetupStatus | null {
	if (activeInstalls[server.id]) return 'installing'
	if (server.installState === 'incomplete') return 'interrupted'
	if (server.installState === 'failed') return 'failed'
	return null
}

/**
 * Runs a modpack server install in the background: progress and log events are
 * tracked in the shared registry so any surface (wizard, servers list, detail
 * page) can render live state, and callers only await completion.
 */
export async function startModpackServerInstall(
	serverId: string,
	options: InstallModpackOptions,
): Promise<void> {
	if (activeInstalls[serverId]) {
		throw new Error('This server already has an install running')
	}
	const entry: ActiveServerInstall = { progress: null, log: [] }
	activeInstalls[serverId] = entry

	const unlistenProgress = await serverEventListener((id, payload) => {
		if (id !== serverId || payload.event !== 'download_progress') return
		entry.progress = { downloaded: payload.downloaded, total: payload.total ?? null }
	})
	const unlistenLogs = await serverEventListener((id, payload) => {
		if (id !== serverId || payload.event !== 'log') return
		entry.log.push(payload.line)
		if (entry.log.length > LOG_CAPACITY) entry.log.splice(0, entry.log.length - LOG_CAPACITY)
	})
	try {
		await servers.installModpack(serverId, options)
	} finally {
		unlistenProgress()
		unlistenLogs()
		activeInstalls[serverId] = undefined
		void refreshServerList()
	}
}

/**
 * Resumes an interrupted or failed modpack install for a server created from a
 * modpack: re-resolves the pack file and launcher jar from the recorded source
 * project, then reruns the same background install.
 */
export async function resumeModpackInstall(server: ServerInfoData): Promise<void> {
	const modpack = server.modpack
	if (!modpack?.versionId) {
		throw new Error('This server has no modpack source to resume from')
	}
	const version = (await get_version(modpack.versionId)) as Labrinth.Versions.v2.Version
	const primaryFile = version.files.find((file) => file.primary) ?? version.files[0]
	if (!primaryFile?.url) {
		throw new Error('Modpack has no downloadable file')
	}
	const jar = await resolveServerLauncher(
		server.serverType,
		server.gameVersion,
		server.loaderVersion,
	)
	if (!jar) {
		throw new Error(
			`No server launcher available for ${server.serverType} on ${server.gameVersion}`,
		)
	}
	await startModpackServerInstall(server.id, {
		mrpackUrl: primaryFile.url,
		mrpackSha1: primaryFile.hashes?.sha1,
		jarUrl: jar.url,
		jarFilename: jar.filename,
		jarSha1: jar.sha1,
		modpackProjectId: modpack.projectId,
		modpackVersionId: modpack.versionId,
		modpackTitle: modpack.title,
		modpackIconUrl: modpack.iconUrl,
	})
}
