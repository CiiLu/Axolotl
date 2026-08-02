import { reactive } from 'vue'

import type { Skin } from '../skins'
import { get_normalized_skin_texture } from '../skins'
import { headStorage } from '../storage/head-storage'
import { skinPreviewStorage } from '../storage/skin-preview-storage'

export interface RenderResult {
	forwards: string
}

export interface RawRenderResult {
	forwards: Blob
}

export const skinBlobUrlMap = reactive(new Map<string, RenderResult>())
export const headBlobUrlMap = reactive(new Map<string, string>())

export async function cleanupUnusedPreviews(skins: Skin[]): Promise<void> {
	const validKeys = new Set<string>()
	const validHeadKeys = new Set<string>()

	for (const skin of skins) {
		const key = `${skin.texture_key}+${skin.variant}+${skin.cape_id ?? 'no-cape'}`
		const headKey = `${skin.texture_key}-head`
		validKeys.add(key)
		validHeadKeys.add(headKey)
	}

	try {
		await skinPreviewStorage.cleanupInvalidKeys(validKeys)
		await headStorage.cleanupInvalidKeys(validHeadKeys)
	} catch (error) {
		console.warn('Failed to cleanup unused skin previews:', error)
	}
}

export async function generatePlayerHeadBlob(skinUrl: string, size: number = 64): Promise<Blob> {
	return new Promise((resolve, reject) => {
		const img = new Image()
		img.crossOrigin = 'anonymous'

		img.onload = () => {
			try {
				const sourceCanvas = document.createElement('canvas')
				const sourceCtx = sourceCanvas.getContext('2d')

				if (!sourceCtx) {
					throw new Error('Could not get 2D context from source canvas')
				}

				sourceCanvas.width = img.width
				sourceCanvas.height = img.height

				sourceCtx.drawImage(img, 0, 0)

				const outputCanvas = document.createElement('canvas')
				const outputCtx = outputCanvas.getContext('2d')

				if (!outputCtx) {
					throw new Error('Could not get 2D context from output canvas')
				}

				outputCanvas.width = size
				outputCanvas.height = size

				outputCtx.imageSmoothingEnabled = false

				const headImageData = sourceCtx.getImageData(8, 8, 8, 8)

				const headCanvas = document.createElement('canvas')
				const headCtx = headCanvas.getContext('2d')

				if (!headCtx) {
					throw new Error('Could not get 2D context from head canvas')
				}

				headCanvas.width = 8
				headCanvas.height = 8
				headCtx.putImageData(headImageData, 0, 0)

				outputCtx.drawImage(headCanvas, 0, 0, 8, 8, 0, 0, size, size)

				const hatImageData = sourceCtx.getImageData(40, 8, 8, 8)

				const hatCanvas = document.createElement('canvas')
				const hatCtx = hatCanvas.getContext('2d')

				if (!hatCtx) {
					throw new Error('Could not get 2D context from hat canvas')
				}

				hatCanvas.width = 8
				hatCanvas.height = 8
				hatCtx.putImageData(hatImageData, 0, 0)

				const hatPixels = hatImageData.data
				let hasHat = false

				for (let i = 3; i < hatPixels.length; i += 4) {
					if (hatPixels[i] > 0) {
						hasHat = true
						break
					}
				}

				if (hasHat) {
					outputCtx.drawImage(hatCanvas, 0, 0, 8, 8, 0, 0, size, size)
				}

				outputCanvas.toBlob(
					(blob) => {
						if (blob) {
							resolve(blob)
						} else {
							reject(new Error('Failed to create blob from canvas'))
						}
					},
					'image/webp',
					0.9,
				)
			} catch (error) {
				reject(error)
			}
		}

		img.onerror = () => {
			reject(new Error('Failed to load skin texture image'))
		}

		img.src = skinUrl
	})
}

export async function generateHeadRender(skin: Skin): Promise<string> {
	const headKey = `${skin.texture_key}-head`

	if (headBlobUrlMap.has(headKey)) {
		if (DEBUG_MODE) {
			const url = headBlobUrlMap.get(headKey)!
			URL.revokeObjectURL(url)
			headBlobUrlMap.delete(headKey)
		} else {
			return headBlobUrlMap.get(headKey)!
		}
	}

	const skinUrl = await get_normalized_skin_texture(skin)
	const headBlob = await generatePlayerHeadBlob(skinUrl, 64)
	const headUrl = URL.createObjectURL(headBlob)

	headBlobUrlMap.set(headKey, headUrl)

	try {
		await headStorage.store(headKey, headBlob)
	} catch (error) {
		console.warn('Failed to store head render in persistent storage:', error)
	}

	return headUrl
}

export async function getPlayerHeadUrl(skin: Skin): Promise<string> {
	return await generateHeadRender(skin)
}
