import { AbstractModule } from '../../../core/abstract-module'
import type { Logshare } from '../types'

const API_BASE = 'https://api.logshare.cn'

export class LogshareLimitsV1Module extends AbstractModule {
	public getModuleID(): string {
		return 'logshare_limits_v1'
	}

	public async getLimits(): Promise<Logshare.Limits.v1.Limits> {
		return this.client.request<Logshare.Limits.v1.Limits>('/limits', {
			api: API_BASE,
			version: 'v1',
			method: 'GET',
			skipAuth: true,
		})
	}

	public async getFilters(): Promise<Logshare.Limits.v1.Filters> {
		return this.client.request<Logshare.Limits.v1.Filters>('/filters', {
			api: API_BASE,
			version: 'v1',
			method: 'GET',
			skipAuth: true,
		})
	}
}
