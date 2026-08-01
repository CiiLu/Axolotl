import { provideFileDrop } from '@modrinth/ui'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { getCurrentWebview } from '@tauri-apps/api/webview'

function toLogicalPosition(position: { x: number; y: number }) {
	const scale = window.devicePixelRatio || 1
	return {
		x: position.x / scale,
		y: position.y / scale,
	}
}

export function setupFileDropProvider() {
	let nativeFileDropPaths: string[] = []

	const provider = {
		async listenNativeFileDrop(handler) {
			return await getCurrentWebview().onDragDropEvent((event: { payload: DragDropEvent }) => {
				const payload = event.payload

				if (payload.type === 'leave') {
					nativeFileDropPaths = []
					void handler({
						type: 'leave',
						paths: [],
						position: { x: 0, y: 0 },
					})
					return
				}

				if (payload.type === 'enter') {
					nativeFileDropPaths = payload.paths
				} else if (payload.type === 'drop' && payload.paths?.length) {
					nativeFileDropPaths = payload.paths
				}

				void handler({
					type: payload.type,
					paths: nativeFileDropPaths,
					position: toLogicalPosition(payload.position),
				})

				if (payload.type === 'drop') {
					nativeFileDropPaths = []
				}
			})
		},
	}

	provideFileDrop(provider)
	return provider
}
