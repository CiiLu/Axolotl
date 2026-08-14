import { AbstractModule } from '../../../core/abstract-module'
import type { Logshare } from '../types'

const API_BASE = 'https://api.logshare.cn'

export class LogshareInsightsV1Module extends AbstractModule {
	public getModuleID(): string {
		return 'logshare_insights_v1'
	}

	public async analyse(content: string): Promise<Logshare.Insights.v1.InsightsResponse> {
		return this.client.request<Logshare.Insights.v1.InsightsResponse>('/analyse', {
			api: API_BASE,
			version: 'v1',
			method: 'POST',
			body: { content },
			headers: { 'Content-Type': 'application/json' },
			skipAuth: true,
		})
	}

	public async get(id: string): Promise<Logshare.Insights.v1.InsightsResponse> {
		return this.client.request<Logshare.Insights.v1.InsightsResponse>(`/insights/${id}`, {
			api: API_BASE,
			version: 'v1',
			method: 'GET',
			skipAuth: true,
		})
	}
}
