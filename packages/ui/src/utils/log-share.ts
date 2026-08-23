import type { AbstractModrinthClient } from '@modrinth/api-client'

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

/**
 * Maximum size in bytes of the log content sent to a sharing service. Logs
 * larger than this are trimmed to their last `LOG_SHARE_MAX_BYTES` before
 * uploading, and the caller is notified through `LogShareResult.truncated`.
 */
export const LOG_SHARE_MAX_BYTES = 9 * 1024 * 1024

/**
 * Return the tail of `content` that fits within `maxBytes` of UTF-8 without
 * splitting a multi-byte code point. Returns the original string when it
 * already fits.
 */
export function truncateLogToMaxBytes(content: string, maxBytes: number): string {
	if (!content) return content
	const bytes = textEncoder.encode(content)
	if (bytes.byteLength <= maxBytes) return content

	// Keep the last `maxBytes` bytes, starting at the first byte that begins a
	// UTF-8 code point so decoding never produces replacement characters.
	let start = bytes.byteLength - maxBytes
	while (start < bytes.byteLength && (bytes[start] & 0xc0) === 0x80) start++

	return textDecoder.decode(bytes.subarray(start))
}

/**
 * Which log analysis / sharing backend to use.
 *
 * - `auto`: prefer logshare.cn, falling back to mclo.gs when unavailable.
 * - `logshare`: prefer logshare.cn, falling back to mclo.gs when unavailable
 *   so that basic log sharing always works.
 * - `mclogs`: always use mclo.gs.
 *
 * AI analysis is only provided by logshare.cn; mclo.gs has no AI endpoint.
 */
export type LogShareProvider = 'auto' | 'mclogs' | 'logshare'

const LOG_SHARE_PROVIDER_KEY = 'axolotl-log-share-provider'

export function getLogShareProvider(): LogShareProvider {
	if (typeof window === 'undefined') return 'auto'
	const value = localStorage.getItem(LOG_SHARE_PROVIDER_KEY)
	return value === 'mclogs' || value === 'logshare' ? value : 'auto'
}

export function setLogShareProvider(provider: LogShareProvider): void {
	if (typeof window === 'undefined') return
	localStorage.setItem(LOG_SHARE_PROVIDER_KEY, provider)
}

function preferredProviders(): LogShareProvider[] {
	const configured = getLogShareProvider()
	if (configured === 'mclogs') return ['mclogs']
	// logshare is always preferred and falls back to mclo.gs so sharing keeps working.
	return ['logshare', 'mclogs']
}

/**
 * Result of sharing log content. `id` is present when the log was uploaded to
 * logshare.cn and can be reused for AI analysis without re-uploading.
 * `truncated` is set when the content exceeded `LOG_SHARE_MAX_BYTES` and only
 * its tail was uploaded.
 */
export type LogShareResult = {
	url: string
	id?: string
	provider: 'mclogs' | 'logshare'
	truncated?: boolean
}

/**
 * Share log content, preferring logshare.cn and falling back to mclo.gs so a
 * shareable link is produced whenever at least one service is reachable.
 * Content larger than `LOG_SHARE_MAX_BYTES` is trimmed to its last
 * `LOG_SHARE_MAX_BYTES` before uploading.
 */
export async function shareLogs(
	client: AbstractModrinthClient,
	content: string,
): Promise<LogShareResult> {
	const uploadContent = truncateLogToMaxBytes(content, LOG_SHARE_MAX_BYTES)
	const truncated = uploadContent !== content
	let lastError: unknown
	for (const provider of preferredProviders()) {
		try {
			if (provider === 'logshare') {
				const data = await client.logshare.logs_v1.create(uploadContent, 'axolotl')
				return { url: data.url, id: data.id, provider: 'logshare', truncated }
			}
			const data = await client.mclogs.logs_v1.create(uploadContent)
			if (data.success && data.url) return { url: data.url, provider: 'mclogs', truncated }
			throw new Error('mclo.gs upload failed')
		} catch (error) {
			lastError = error
		}
	}
	throw lastError ?? new Error('Failed to share logs')
}
