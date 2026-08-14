import { AbstractModule } from '../../../core/abstract-module'
import type { Logshare } from '../types'

const API_BASE = 'https://api.logshare.cn'

/**
 * Consume a logshare.cn AI SSE stream.
 *
 * The stream follows the OpenAI chunk format: `data: {json}` lines where each
 * payload carries `choices[0].delta.content`, terminated by an `event: done`
 * line or an EOF.
 */
async function consumeAIStream(
	stream: ReadableStream<Uint8Array>,
	handlers: Logshare.AI.v1.StreamHandlers,
	signal?: AbortSignal,
): Promise<void> {
	const reader = stream.getReader()
	const decoder = new TextDecoder()
	let buffer = ''
	let fullContent = ''
	let done = false
	let aborted = false

	const onAbort = () => {
		aborted = true
		void reader.cancel()
	}
	signal?.addEventListener('abort', onAbort, { once: true })

	try {
		while (!done) {
			const { value, done: streamDone } = await reader.read()
			if (streamDone) break
			buffer += decoder.decode(value, { stream: true })

			const lines = buffer.split('\n')
			buffer = lines.pop() ?? ''
			for (const line of lines) {
				if (line.startsWith('event: done')) {
					done = true
					break
				}
				if (!line.startsWith('data: ')) continue
				const data = line.slice(6).trim()
				if (!data || data === '[DONE]') continue
				try {
					const parsed = JSON.parse(data) as {
						choices?: Array<{ delta?: { content?: string } }>
					}
					const content = parsed.choices?.[0]?.delta?.content
					if (typeof content === 'string' && content.length > 0) {
						fullContent += content
						handlers.onChunk?.(content)
					}
				} catch {
					// Skip malformed JSON chunks instead of aborting the stream
				}
			}
		}
		if (!aborted) handlers.onDone?.(fullContent)
	} catch (error) {
		if (!aborted) {
			handlers.onError?.({
				success: false,
				message: error instanceof Error ? error.message : String(error),
				type: 'stream_error',
			})
		}
	} finally {
		signal?.removeEventListener('abort', onAbort)
		reader.releaseLock()
	}
}

export class LogshareAIV1Module extends AbstractModule {
	public getModuleID(): string {
		return 'logshare_ai_v1'
	}

	/**
	 * Stream an AI analysis of log content without storing it. Responses are
	 * cached server-side by content hash for 30 minutes.
	 */
	public async analyseStream(
		content: string,
		handlers: Logshare.AI.v1.StreamHandlers,
		signal?: AbortSignal,
	): Promise<void> {
		let stream: ReadableStream<Uint8Array>
		try {
			stream = await this.client.stream('/ai/analyse', {
				api: API_BASE,
				version: 'v1',
				method: 'POST',
				body: { content },
				headers: { 'Content-Type': 'application/json' },
				skipAuth: true,
				signal,
			})
		} catch (error) {
			handlers.onError?.(errorToStreamError(error))
			return
		}
		await consumeAIStream(stream, handlers, signal)
	}

	/**
	 * Stream an AI analysis of an already-stored log.
	 */
	public async getStream(
		id: string,
		handlers: Logshare.AI.v1.StreamHandlers,
		signal?: AbortSignal,
	): Promise<void> {
		let stream: ReadableStream<Uint8Array>
		try {
			stream = await this.client.stream(`/ai/${id}`, {
				api: API_BASE,
				version: 'v1',
				method: 'GET',
				skipAuth: true,
				signal,
			})
		} catch (error) {
			handlers.onError?.(errorToStreamError(error))
			return
		}
		await consumeAIStream(stream, handlers, signal)
	}
}

function errorToStreamError(error: unknown): Logshare.AI.v1.StreamError {
	const statusCode = (error as { statusCode?: number })?.statusCode
	const type = statusCode === 429 ? 'rate_limit' : statusCode === 404 ? 'not_found' : 'server_error'
	const message =
		error instanceof Error
			? error.message
			: typeof error === 'string'
				? error
				: 'Failed to reach the log analysis service'
	return { success: false, message, code: statusCode, type }
}
