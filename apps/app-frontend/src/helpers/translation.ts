import { renderHighlightedString } from '@modrinth/utils'
import { configuredXss } from '@modrinth/utils/parse'
import { invoke } from '@tauri-apps/api/core'

export interface TranslatableHit {
	project_id?: string
	id?: string
	title?: string
	description?: string
	name?: string
	summary?: string
	provider?: 'modrinth' | 'curseforge'
	provider_project_id?: string
}

export type TranslationProvider = 'microsoft' | 'google' | 'openai-compatible'
export type TranslationMode = 'bilingual' | 'translation-only'
export type TranslationStyle = 'default' | 'weakened' | 'brand' | 'border' | 'background'
export type TranslationTextFormat = 'plain' | 'html'
export type DescriptionSourceFormat = 'markdown' | 'html'

export interface TranslationSettings {
	provider: TranslationProvider
	target_language: string
	mode: TranslationMode
	auto_translate: boolean
	style: TranslationStyle
	openai_base_url: string
	openai_model: string
	openai_has_api_key: boolean
	openai_system_prompt: string
}

export interface TranslationSegment {
	id: string
	text: string
	format: TranslationTextFormat
}

export interface TranslationRequest {
	source_language: string
	target_language: string
	context: {
		title: string
		description: string
	}
	segments: TranslationSegment[]
}

export interface TranslationResponse {
	segments: Array<{ id: string; text: string }>
}

interface ProtectedElement {
	tagName: string
	attributes: Array<[string, string]>
	innerHtml?: string
}

export interface PreparedDescriptionBlock {
	id: string
	originalHtml: string
	translatable: boolean
	protectedElements: Record<string, ProtectedElement>
}

export interface PreparedDescription {
	blocks: PreparedDescriptionBlock[]
	segments: TranslationSegment[]
}

export async function getTranslationSettings(): Promise<TranslationSettings> {
	return await invoke('plugin:translation|translation_get_settings')
}

export async function updateTranslationSettings(settings: TranslationSettings): Promise<void> {
	await invoke('plugin:translation|translation_update_settings', { settings })
}

export async function setTranslationSecret(
	provider: TranslationProvider,
	secret: string | null,
): Promise<void> {
	await invoke('plugin:translation|translation_set_secret', { provider, secret })
}

export async function testTranslationProvider(provider: TranslationProvider): Promise<string> {
	return await invoke('plugin:translation|translation_test_provider', { provider })
}

export async function translate(request: TranslationRequest): Promise<TranslationResponse> {
	return await invoke('plugin:translation|translation_translate', { request })
}

export async function clearTranslationCache(): Promise<void> {
	await invoke('plugin:translation|translation_clear_cache')
}

export type TranslationErrorKind =
	| 'rate-limited'
	| 'authentication'
	| 'content-too-long'
	| 'network'
	| 'provider'

function translationErrorMessage(error: unknown): string {
	if (error instanceof Error) return error.message
	if (typeof error === 'string') return error
	if (
		typeof error === 'object' &&
		error !== null &&
		'message' in error &&
		typeof error.message === 'string'
	) {
		return error.message
	}
	return String(error)
}

export function getTranslationErrorKind(error: unknown): TranslationErrorKind {
	const message = translationErrorMessage(error)
	if (message.includes('TRANSLATION_RATE_LIMITED')) return 'rate-limited'
	if (message.includes('TRANSLATION_AUTHENTICATION_FAILED')) return 'authentication'
	if (message.includes('TRANSLATION_CONTENT_TOO_LONG')) return 'content-too-long'
	if (message.includes('TRANSLATION_NETWORK_FAILED')) return 'network'
	return 'provider'
}

function containsReadableText(element: Element): boolean {
	if (element.matches('pre, script, style, video, audio, iframe')) return false
	const clone = element.cloneNode(true) as Element
	clone.querySelectorAll('pre, code, script, style').forEach((node) => node.remove())
	clone.querySelectorAll('a').forEach((node) => {
		if (isUrlOnlyText(node.textContent ?? '')) node.remove()
	})
	return (clone.textContent ?? '').trim().length > 0
}

function isUrlOnlyText(value: string): boolean {
	return /^(?:https?:\/\/|www\.|mailto:)[^\s]+$/i.test(value.trim())
}

function protectElementAttributes(
	element: Element,
	blockIndex: number,
): Record<string, ProtectedElement> {
	const protectedElements: Record<string, ProtectedElement> = {}
	const elements = [element, ...Array.from(element.querySelectorAll('*'))]

	elements.forEach((current, elementIndex) => {
		const marker = `${blockIndex}-${elementIndex}`
		const attributes = Array.from(current.attributes).map(
			(attribute) => [attribute.name, attribute.value] as [string, string],
		)
		protectedElements[marker] = {
			tagName: current.tagName,
			attributes,
			...(current.matches('code, pre') ||
			(current.matches('a') && isUrlOnlyText(current.textContent ?? ''))
				? { innerHtml: current.innerHTML }
				: {}),
		}

		Array.from(current.attributes).forEach((attribute) => current.removeAttribute(attribute.name))
		current.setAttribute('data-ax-translation-attr', marker)
		if (protectedElements[marker].innerHtml !== undefined) current.setAttribute('translate', 'no')
	})

	return protectedElements
}

export function prepareDescription(
	description: string,
	sourceFormat: DescriptionSourceFormat = 'markdown',
): PreparedDescription {
	const renderedDescription =
		sourceFormat === 'html'
			? configuredXss.process(description ?? '')
			: renderHighlightedString(description ?? '')
	const document = new DOMParser().parseFromString(
		`<body>${renderedDescription}</body>`,
		'text/html',
	)
	const blocks: PreparedDescriptionBlock[] = []
	const segments: TranslationSegment[] = []

	Array.from(document.body.children).forEach((source, index) => {
		const id = `body-${index}`
		const originalHtml = configuredXss.process(source.outerHTML)
		const translatable = containsReadableText(source)
		const clone = source.cloneNode(true) as Element
		const protectedElements = translatable ? protectElementAttributes(clone, index) : {}

		blocks.push({ id, originalHtml, translatable, protectedElements })
		if (translatable) {
			segments.push({ id, text: clone.outerHTML, format: 'html' })
		}
	})

	return { blocks, segments }
}

function restoreTranslatedBlock(block: PreparedDescriptionBlock, translatedHtml: string): string {
	const document = new DOMParser().parseFromString(`<body>${translatedHtml}</body>`, 'text/html')
	const root = document.body.firstElementChild
	const translatedElements = document.body.querySelectorAll('*')
	if (
		!root ||
		document.body.children.length !== 1 ||
		translatedElements.length !== Object.keys(block.protectedElements).length ||
		Array.from(translatedElements).some(
			(element) => !element.hasAttribute('data-ax-translation-attr'),
		)
	) {
		throw new Error(`Translation markup changed for block ${block.id}`)
	}

	for (const [marker, protectedElement] of Object.entries(block.protectedElements)) {
		const matches = document.body.querySelectorAll(`[data-ax-translation-attr="${marker}"]`)
		if (matches.length !== 1 || matches[0].tagName !== protectedElement.tagName) {
			throw new Error(`Translation markup changed for block ${block.id}`)
		}
		const element = matches[0]
		Array.from(element.attributes).forEach((attribute) => element.removeAttribute(attribute.name))
		protectedElement.attributes.forEach(([name, value]) => element.setAttribute(name, value))
		if (protectedElement.innerHtml !== undefined) element.innerHTML = protectedElement.innerHtml
	}

	return configuredXss.process(root.outerHTML)
}

function translationStyleClass(style: TranslationStyle): string {
	return `ax-translation-style-${style}`
}

function restorePreparedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
): Map<string, string> {
	const restored = new Map<string, string>()
	for (const block of prepared.blocks) {
		if (!block.translatable) continue
		const translated = translations[block.id]
		if (!translated) throw new Error(`Missing translated block ${block.id}`)
		restored.set(block.id, restoreTranslatedBlock(block, translated))
	}
	return restored
}

export function validateTranslatedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
): void {
	restorePreparedDescription(prepared, translations)
}

export function renderTranslatedDescription(
	prepared: PreparedDescription,
	translations: Record<string, string>,
	mode: TranslationMode,
	style: TranslationStyle,
): string {
	let restored: Map<string, string>
	try {
		restored = restorePreparedDescription(prepared, translations)
	} catch {
		return prepared.blocks.map((block) => block.originalHtml).join('')
	}

	return prepared.blocks
		.map((block) => {
			if (!block.translatable) return block.originalHtml
			const translated = restored.get(block.id) ?? block.originalHtml
			if (mode === 'translation-only') return translated
			return `${block.originalHtml}<div class="ax-translation-block ${translationStyleClass(style)}">${translated}</div>`
		})
		.join('')
}

const MIRROR_API_BASE = 'https://mod.mcimirror.top/translate'

/** Cache: key → translated description string. Key format: `cf:{provider_project_id}` or `mr:{project_id}`. */
const descriptionCache = new Map<string, string>()

function mirrorCacheKey(hit: TranslatableHit): string | null {
	if (hit.provider === 'curseforge') {
		const id = hit.provider_project_id
		if (!id) return null
		return `cf:${id}`
	}
	if (hit.provider === 'modrinth') {
		const id = hit.project_id
		if (!id) return null
		return `mr:${id}`
	}
	return null
}

interface MirrorTranslationResponse {
	modid?: number
	project_id?: string
	translated: string
	original: string
	translated_at: string
}

/** Fetch translated description for a single project from the mcimirror API. */
async function fetchMirrorDescription(hit: TranslatableHit): Promise<string | null> {
	const cacheKey = mirrorCacheKey(hit)
	if (!cacheKey) return null

	const cached = descriptionCache.get(cacheKey)
	if (cached !== undefined) return cached || null

	const id = hit.provider === 'curseforge' ? hit.provider_project_id : hit.project_id

	if (!id || !hit.provider) return null

	const url = `${MIRROR_API_BASE}/${hit.provider}/${encodeURIComponent(id)}`

	try {
		const response = await fetch(url)
		if (!response.ok) return null
		const data = (await response.json()) as MirrorTranslationResponse
		const translated = data.translated?.trim() || null
		descriptionCache.set(cacheKey, translated ?? '')
		return translated
	} catch {
		descriptionCache.set(cacheKey, '')
		return null
	}
}

/**
 * Rewrites search hit descriptions to Chinese using the mcimirror translation
 * API. Each hit is fetched in parallel; only the description field is replaced.
 * Hits without a translatable description are left unchanged. Results are
 * cached in memory across searches.
 *
 * @param hits  Search result hits that carry at least provider + ID and
 *              description fields.
 * @param locale Target locale — only `zh-CN` triggers translation.
 * @param force Ignored (kept for API compatibility with the old translateSearchHits).
 */
export async function translateSearchDescriptions<T extends TranslatableHit>(
	hits: T[],
	locale: string,
	_force = false,
): Promise<T[]> {
	if (locale !== 'zh-CN' || hits.length === 0) return hits

	const entries = hits.map((hit) => ({ hit, index: hits.indexOf(hit) }))

	const results = await Promise.allSettled(
		entries.map(async ({ hit, index }) => {
			const originalDesc = hit.description ?? hit.summary ?? ''
			if (!originalDesc) return { index, hit }

			const translation = await fetchMirrorDescription(hit)
			if (!translation) return { index, hit }

			return {
				index,
				hit: {
					...hit,
					description: translation,
					summary: translation,
				} as T,
			}
		}),
	)

	const translatedHits = [...hits]
	for (const result of results) {
		if (result.status === 'rejected') continue
		translatedHits[result.value.index] = result.value.hit
	}

	return translatedHits
}
