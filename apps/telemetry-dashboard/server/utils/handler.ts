import {
	createError,
	defineEventHandler,
	type EventHandler,
	type EventHandlerRequest,
	isError,
} from 'h3'

import { AdminApiError } from './errors'

export function defineAdminHandler<T>(
	handler: EventHandler<EventHandlerRequest, T | Promise<T>>,
): EventHandler<EventHandlerRequest, Promise<T>> {
	return defineEventHandler(async (event) => {
		try {
			return await handler(event)
		} catch (error) {
			if (error instanceof AdminApiError) {
				throw createError({
					statusCode: error.statusCode,
					statusMessage: error.message,
					data: { error: { code: error.code, message: error.message } },
				})
			}
			if (isError(error) && 'statusCode' in error && Number(error.statusCode) < 500) throw error
			throw createError({
				statusCode: 503,
				statusMessage: '遥测数据暂不可用',
				data: {
					error: {
						code: 'service_unavailable',
						message: '遥测数据暂不可用',
					},
				},
			})
		}
	})
}
