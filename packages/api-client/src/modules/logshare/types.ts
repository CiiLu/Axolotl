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
