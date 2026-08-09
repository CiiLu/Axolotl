export interface DownloadsScanLoopOptions<T> {
	scan: () => Promise<T>
	onResult: (result: T) => void
	onError?: (error: unknown) => void
	onScanningChange?: (scanning: boolean) => void
	intervalMs?: number
	schedule?: (callback: () => void, delay: number) => unknown
	cancelSchedule?: (timer: unknown) => void
}

export type DownloadsScannerPresentationPhase =
	| 'idle'
	| 'scanning'
	| 'verifying'
	| 'waiting_for_stability'
	| 'rejected'
	| 'imported'
	| 'error'
	| 'watching'
	| 'unavailable'

export interface DownloadsScannerPresentationState {
	phase: DownloadsScannerPresentationPhase
	downloadDirectory: string | null
	importedCount: number
	pendingCandidates: number
	rejectedItemIds: string[]
	verifyingItemIds: string[]
}

export type DownloadsScannerPresentationEvent =
	| { type: 'reset' }
	| { type: 'scan_started' }
	| { type: 'scan_finished' }
	| { type: 'scan_failed' }
	| {
			type: 'items_updated'
			items: Array<{ id: string; status: string }>
	  }
	| {
			type: 'scan_result'
			downloadDirectory: string | null
			importedItemIds: string[]
			mismatchedItemIds: string[]
			pendingCandidates: number
			hasErrors: boolean
			items: Array<{ id: string; status: string }>
	  }
	| { type: 'items_resolved'; itemIds: string[] }

export function createDownloadsScannerPresentationState(): DownloadsScannerPresentationState {
	return {
		phase: 'idle',
		downloadDirectory: null,
		importedCount: 0,
		pendingCandidates: 0,
		rejectedItemIds: [],
		verifyingItemIds: [],
	}
}

function candidateProcessingItemIds(items: Array<{ id: string; status: string }>) {
	return items
		.filter((item) => item.status === 'verifying' || item.status === 'writing')
		.map((item) => item.id)
}

function withoutItemIds(current: string[], removed: string[]) {
	const removedSet = new Set(removed)
	return current.filter((itemId) => !removedSet.has(itemId))
}

function withPhase(
	state: Omit<DownloadsScannerPresentationState, 'phase'>,
	options: { scanning?: boolean; failed?: boolean } = {},
): DownloadsScannerPresentationState {
	let phase: DownloadsScannerPresentationPhase
	if (state.verifyingItemIds.length > 0) phase = 'verifying'
	else if (state.pendingCandidates > 0) phase = 'waiting_for_stability'
	else if (state.rejectedItemIds.length > 0) phase = 'rejected'
	else if (options.scanning) phase = 'scanning'
	else if (state.importedCount > 0) phase = 'imported'
	else if (options.failed) phase = 'error'
	else if (state.downloadDirectory) phase = 'watching'
	else phase = 'unavailable'
	return { ...state, phase }
}

export function reduceDownloadsScannerPresentation(
	state: DownloadsScannerPresentationState,
	event: DownloadsScannerPresentationEvent,
): DownloadsScannerPresentationState {
	if (event.type === 'reset') return createDownloadsScannerPresentationState()

	const current = {
		downloadDirectory: state.downloadDirectory,
		importedCount: state.importedCount,
		pendingCandidates: state.pendingCandidates,
		rejectedItemIds: [...state.rejectedItemIds],
		verifyingItemIds: [...state.verifyingItemIds],
	}

	if (event.type === 'scan_started') return withPhase(current, { scanning: true })
	if (event.type === 'scan_finished') {
		if (state.phase === 'idle' || state.phase === 'error') return state
		return withPhase(current)
	}
	if (event.type === 'scan_failed') return withPhase(current, { failed: true })

	if (event.type === 'items_updated') {
		const verifyingItemIds = candidateProcessingItemIds(event.items)
		if (verifyingItemIds.length === 0) return state
		current.verifyingItemIds = verifyingItemIds
		current.rejectedItemIds = withoutItemIds(current.rejectedItemIds, verifyingItemIds)
		return withPhase(current)
	}

	if (event.type === 'items_resolved') {
		current.pendingCandidates = 0
		current.rejectedItemIds = withoutItemIds(current.rejectedItemIds, event.itemIds)
		current.verifyingItemIds = withoutItemIds(current.verifyingItemIds, event.itemIds)
		return withPhase(current)
	}

	current.downloadDirectory = event.downloadDirectory
	current.pendingCandidates = event.pendingCandidates
	current.verifyingItemIds = candidateProcessingItemIds(event.items)
	if (event.pendingCandidates > 0) current.rejectedItemIds = []
	current.rejectedItemIds = withoutItemIds(current.rejectedItemIds, [
		...event.importedItemIds,
		...current.verifyingItemIds,
	])
	current.rejectedItemIds = [...new Set([...current.rejectedItemIds, ...event.mismatchedItemIds])]
	current.importedCount += event.importedItemIds.length
	return withPhase(current, { failed: event.hasErrors })
}

export function createDownloadsScanLoop<T>(options: DownloadsScanLoopOptions<T>) {
	const schedule = options.schedule ?? ((callback, delay) => setTimeout(callback, delay))
	const cancelSchedule = options.cancelSchedule ?? ((timer) => clearTimeout(timer as number))
	const intervalMs = options.intervalMs ?? 3000
	let active = false
	let generation = 0
	let inFlight = false
	let timer: unknown

	function clearTimer() {
		if (timer != null) cancelSchedule(timer)
		timer = undefined
	}

	function scheduleNext(delay: number) {
		if (!active) return
		clearTimer()
		timer = schedule(() => {
			timer = undefined
			void runNow()
		}, delay)
	}

	function start() {
		stop()
		active = true
		generation += 1
		scheduleNext(0)
	}

	function stop() {
		active = false
		generation += 1
		clearTimer()
		options.onScanningChange?.(false)
	}

	async function runNow() {
		if (!active) return
		if (inFlight) {
			scheduleNext(100)
			return
		}
		const runGeneration = generation
		inFlight = true
		options.onScanningChange?.(true)
		try {
			const result = await options.scan()
			if (active && runGeneration === generation) options.onResult(result)
		} catch (error) {
			if (active && runGeneration === generation) options.onError?.(error)
		} finally {
			inFlight = false
			if (active && runGeneration === generation) {
				options.onScanningChange?.(false)
				scheduleNext(intervalMs)
			}
		}
	}

	return {
		start,
		stop,
		runNow,
		isActive: () => active,
	}
}
