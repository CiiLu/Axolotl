import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export function readStudioText(instanceId: string, filePath: string): Promise<string> {
	return invoke('plugin:files|studio_read_text', { instanceId, filePath })
}

export function trashStudioFile(instanceId: string, filePath: string): Promise<void> {
	return invoke('plugin:files|studio_trash', { instanceId, filePath })
}

export interface StudioFilesChangedEvent {
	instanceId: string
	registrationId: string
	paths: string[]
}

export function registerStudioWatcher(instanceId: string): Promise<string> {
	return invoke('plugin:files|studio_watch_register', { instanceId })
}

export function unregisterStudioWatcher(instanceId: string, registrationId: string): Promise<void> {
	return invoke('plugin:files|studio_watch_unregister', { instanceId, registrationId })
}

export function listenStudioFilesChanged(
	handler: (event: StudioFilesChangedEvent) => void,
): Promise<UnlistenFn> {
	return listen<StudioFilesChangedEvent>('studio-files-changed', (event) => handler(event.payload))
}
