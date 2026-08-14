import { AbstractModule } from '../../../core/abstract-module'
import type { Logshare } from '../types'

const API_BASE = 'https://api.logshare.cn'

export class LogshareLogsV1Module extends AbstractModule {
	public getModuleID(): string {
		return 'logshare_logs_v1'
	}

	public async create(
		content: string,
		source?: string,
		metadata?: Record<string, string>,
	): Promise<Logshare.Logs.v1.CreateResponse> {
		const body: Logshare.Logs.v1.CreateRequest = { content }
		if (source) body.source = source
		if (metadata && Object.keys(metadata).length > 0) body.metadata = metadata

		return this.client.request<Logshare.Logs.v1.CreateResponse>('/log', {
			api: API_BASE,
			version: 'v1',
			method: 'POST',
			body,
			headers: { 'Content-Type': 'application/json' },
			skipAuth: true,
		})
	}

	public async deleteLog(
		id: string | string[],
		token: string,
	): Promise<Logshare.Logs.v1.DeleteResponse> {
		const ids = Array.isArray(id) ? id.join(',') : id
		return this.client.request<Logshare.Logs.v1.DeleteResponse>(`/log/${ids}`, {
			api: API_BASE,
			version: 'v1',
			method: 'DELETE',
			headers: { Authorization: `Bearer ${token}` },
			skipAuth: true,
		})
	}
}
