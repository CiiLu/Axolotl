export interface DashboardBindings {
	DB?: D1Database
	ERROR_CONTEXTS?: R2Bucket
}

export function dashboardBindings(event: { context: Record<string, unknown> }): DashboardBindings {
	const cloudflare = event.context.cloudflare as { env?: DashboardBindings } | undefined
	return cloudflare?.env ?? (event.context.env as DashboardBindings | undefined) ?? {}
}
