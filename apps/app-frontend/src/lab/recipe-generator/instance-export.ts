// 由 S4 集成
import { join } from '@tauri-apps/api/path'
import { mkdir, writeFile } from '@tauri-apps/plugin-fs'

import { get_full_path } from '@/helpers/instance'

import { createDatapackBlob, type PackFile } from './datapack.ts'

function normalizePackFileName(fileName: string): string {
	const segments = fileName.replaceAll('\\', '/').split('/').filter(Boolean)
	const safeName = segments[segments.length - 1] ?? fileName
	return safeName.toLowerCase().endsWith('.zip') ? safeName : `${safeName}.zip`
}

function normalizeWorldPath(worldPath: string): string {
	const segments = worldPath.replaceAll('\\', '/').split('/').filter(Boolean)
	if (segments.length === 0 || segments.some((segment) => segment === '..')) {
		throw new Error('Invalid world path')
	}
	return segments.join('/')
}

/**
 * Installs a generated datapack into a singleplayer world's `datapacks` directory.
 * Returns the installed path relative to the instance root.
 */
export async function exportDatapackToWorld(
	instanceId: string,
	worldPath: string,
	files: PackFile[],
	fileName = 'axolotl-recipes.zip',
): Promise<string> {
	const instancePath = await get_full_path(instanceId)
	const datapacksPath = await join(
		instancePath,
		'saves',
		normalizeWorldPath(worldPath),
		'datapacks',
	)
	await mkdir(datapacksPath, { recursive: true })

	const blob = createDatapackBlob(files)
	const bytes = new Uint8Array(await blob.arrayBuffer())
	const safeFileName = normalizePackFileName(fileName)
	await writeFile(await join(datapacksPath, safeFileName), bytes)
	return `saves/${normalizeWorldPath(worldPath)}/datapacks/${safeFileName}`
}
