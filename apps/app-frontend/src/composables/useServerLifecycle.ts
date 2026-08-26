import { parseEula, setEulaAccepted } from '@modrinth/server'
import { injectNotificationManager } from '@modrinth/ui'
import { onScopeDispose, ref, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import type EulaModal from '@/components/multiplayer/servers/EulaModal.vue'
import { resumeModpackInstall } from '@/composables/useServerInstalls'
import { type ServerView, setServerExitReasonHandler, useServers } from '@/composables/useServers'
import { serverEventListener,servers as serversApi } from '@/helpers/servers'
import { injectDownloadManager } from '@/providers/download-manager'

/**
 * Shared "start with EULA gate" flow used by the servers overview and the
 * server detail page. Bind the returned `eulaModal` ref to an `<EulaModal>`
 * rendered with `ref="eulaModal"` in the component template.
 */
export function useServerLifecycle() {
	const { startServer } = useServers()
	const { handleError } = injectNotificationManager()

	// [SERVER-DOWNLOAD-BRIDGE] Capture the download manager once during Vue
	// setup context.  See the note in `startModpackServerInstall` for why
	// this must be done here and not later.
	let downloadManager: ReturnType<typeof injectDownloadManager> | null = null
	try {
		downloadManager = injectDownloadManager()
	} catch {
		// Not inside a provider tree — server downloads will not appear in sidebar.
	}

	const eulaModal = useTemplateRef<ComponentExposed<typeof EulaModal>>('eulaModal')
	const eulaText = ref('')
	let pendingId = ''

	// Listen for real-time EULA prompt detection from the Rust backend
	let unregisterEulaListener: (() => void) | null = null
	serverEventListener((serverId, payload) => {
		if (payload.event === 'eula_required' && payload.server_id === pendingId) {
			eulaText.value = payload.eula_text
			eulaModal.value?.show()
		}
	}).then((unregister) => {
		unregisterEulaListener = unregister
	})
	onScopeDispose(() => {
		unregisterEulaListener?.()
	})

	/** Starts the server; if the EULA is unaccepted, shows the EULA modal first. */
	async function tryStartServer(server: ServerView) {
		// Always check the actual eula.txt file on the server
		try {
			const eulaTextContent = await serversApi.readFile(server.id, 'eula.txt')
			const accepted = eulaTextContent.split('\n').some(line => line.trim() === 'eula=true')
			
			if (!accepted) {
				// EULA not accepted, show modal
				eulaText.value = eulaTextContent
				pendingId = server.id
				eulaModal.value?.show()
				return
			}
		} catch {
			// eula.txt doesn't exist yet, will be created by the server
		}
		
		await launchServer(server.id)
	}

	/**
	 * Starts the server and, when the start itself fails over an unaccepted
	 * EULA, falls back to the confirmation dialog instead of just the error.
	 */
	async function launchServer(serverId: string) {
		const started = await startServer(serverId)
		if (started) return
		try {
			const text = await serversApi.readFile(serverId, 'eula.txt')
			if (parseEula(text).accepted) return
			eulaText.value = text
			pendingId = serverId
			eulaModal.value?.show()
		} catch {
			// Start failed for a non-EULA reason; that error was already surfaced.
		}
	}

	async function acceptEula() {
		const id = pendingId
		if (!id) return
		try {
			const updated = setEulaAccepted(eulaText.value, true)
			await serversApi.writeFile(id, 'eula.txt', updated)
			pendingId = ''
			eulaModal.value?.hide()
			// Start the server after accepting EULA
			await startServer(id)
		} catch (error) {
			console.error(error)
		}
	}

	function declineEula() {
		pendingId = ''
		eulaModal.value?.hide()
	}

	/**
	 * Offers the EULA dialog after the server exited on its own over an
	 * unaccepted EULA (detected from the process's final output). Accepting
	 * writes `eula.txt` and restarts, matching the pre-start gate.
	 */
	async function offerEulaAfterExit(serverId: string) {
		try {
			const text = await serversApi.readFile(serverId, 'eula.txt')
			if (parseEula(text).accepted) return
			eulaText.value = text
			pendingId = serverId
			eulaModal.value?.show()
		} catch {
			// No eula.txt to show; the exit stays unexplained.
		}
	}

	const unregisterExitReasonHandler = setServerExitReasonHandler((serverId, reason) => {
		if (reason === 'eula') void offerEulaAfterExit(serverId)
	})
	onScopeDispose(unregisterExitReasonHandler)

	/** Resumes or retries an interrupted/failed modpack download for this server. */
	async function resumeInstall(server: ServerView) {
		try {
			// [SERVER-DOWNLOAD-BRIDGE] Pass the download manager captured
			// during setup so the synthetic job appears in sidebar.
			await resumeModpackInstall(server, downloadManager)
		} catch (error) {
			handleError(error)
		}
	}

	return { eulaModal, eulaText, tryStartServer, acceptEula, declineEula, resumeInstall, offerEulaAfterExit }
}
