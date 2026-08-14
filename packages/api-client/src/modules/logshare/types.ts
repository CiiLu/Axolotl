import type { Mclogs } from '../mclogs/types'

export namespace Logshare {
	export namespace Logs {
		export namespace v1 {
			export type CreateRequest = {
				content: string
				metadata?: Record<string, string>
				source?: string
			}

			export type CreateResponse = {
				success: boolean
				message: string
				id: string
				url: string
				raw: string
				token: string
			}

			export type DeleteResponse = {
				success: boolean
				message?: string
			}
		}
	}

	export namespace Insights {
		export namespace v1 {
			// logshare.cn is mclogs-compatible ("McLogs Next"): /v1/analyse and
			// /v1/insights/{id} return the exact mclogs insights response shape.
			export type InsightsResponse = Mclogs.Insights.v1.InsightsResponse
		}
	}

	export namespace AI {
		export namespace v1 {
			export type StreamError = {
				success: boolean
				message: string
				code?: number
				type?: string
			}

			export type StreamHandlers = {
				onChunk?: (content: string) => void
				onDone?: (fullContent: string) => void
				onError?: (error: StreamError) => void
			}
		}
	}

	export namespace Limits {
		export namespace v1 {
			export type Limits = {
				storageTime: number
				maxLength: number
				maxLines: number
			}

			export type Filter = {
				type: string
				data: unknown
			}

			export type Filters = {
				success: boolean
				filters: Filter[]
			}
		}
	}
}
