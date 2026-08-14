import { env } from 'cloudflare:test'
import { beforeEach, describe, expect, it } from 'vitest'

import { D1TelemetryAdminApi } from '../../server/utils/d1-admin-api'
import { parseErrorQuery } from '../../server/utils/validation'

const NOW = new Date('2026-08-14T10:00:00.000Z')

async function gzip(input: string): Promise<ArrayBuffer> {
	return new Response(
		new Blob([input]).stream().pipeThrough(new CompressionStream('gzip')),
	).arrayBuffer()
}

async function seed(): Promise<void> {
	await env.DB.batch([
		env.DB.prepare(
			`INSERT INTO installations
			(installation_hash, first_seen_at, last_seen_at, first_seen_day, app_version, platform, arch)
			VALUES ('fixture-hash-a', 1, 2, '2026-08-13', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture')`,
		),
		env.DB.prepare(
			`INSERT INTO installations
			(installation_hash, first_seen_at, last_seen_at, first_seen_day, app_version, platform, arch)
			VALUES ('fixture-hash-b', 1, 2, '2026-08-14', '9.3.2-fixture', 'linux-fixture', 'aarch64-fixture')`,
		),
		env.DB.prepare(
			`INSERT INTO daily_active (day, installation_hash, app_version, platform, arch)
			VALUES ('2026-08-14', 'fixture-hash-a', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture')`,
		),
		env.DB.prepare(
			`INSERT INTO daily_active (day, installation_hash, app_version, platform, arch)
			VALUES ('2026-08-14', 'fixture-hash-b', '9.3.2-fixture', 'linux-fixture', 'aarch64-fixture')`,
		),
		env.DB.prepare(
			`INSERT INTO error_reports
			(event_id, installation_hash, day, occurred_at, fingerprint, app_version, platform, arch,
			error_type, message, occurrence_count, object_key, created_at)
			VALUES ('fixture-event-a', 'fixture-hash-a', '2026-08-14', '2026-08-14T08:00:00Z',
			'fixture-render-01', '9.4.0-fixture', 'windows-fixture', 'x86_64-fixture',
			'RenderFixtureError', 'Fixture render failure', 4,
			'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz', 1)`,
		),
		env.DB.prepare(
			`INSERT INTO error_context_reservations
			(event_id, day, fingerprint, app_version, object_key, created_at)
			VALUES ('fixture-event-a', '2026-08-14', 'fixture-render-01', '9.4.0-fixture',
			'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz', 1)`,
		),
	])
	await env.ERROR_CONTEXTS.put(
		'errors/2026-08-14/fixture-render-01/fixture-event-a.json.gz',
		await gzip(
			JSON.stringify({
				occurred_at: '2026-08-14T08:00:00Z',
				fingerprint: 'fixture-render-01',
				app: {
					version: '9.4.0-fixture',
					platform: 'windows-fixture',
					arch: 'x86_64-fixture',
				},
				error_type: 'RenderFixtureError',
				message: 'Fixture render failure',
				stack: 'Fixture stack',
				route: '/fixture',
				command: 'fixture-command',
				context: 'synthetic fixture context',
				installation_hash: 'must-not-leak',
				object_key: 'must-not-leak',
			}),
		),
		{ httpMetadata: { contentEncoding: 'gzip', contentType: 'application/json' } },
	)
}

function api(): D1TelemetryAdminApi {
	return new D1TelemetryAdminApi(env.DB, env.ERROR_CONTEXTS, {
		storeErrorContext: true,
		healthUrl: 'https://fixture.invalid/health',
		fetcher: async () => new Response(JSON.stringify({ status: 'ok' })),
		now: () => NOW,
	})
}

describe('D1TelemetryAdminApi', () => {
	beforeEach(seed)

	it('maps D1 aggregates to stable overview and DAU/WAU/MAU DTOs', async () => {
		const overview = await api().overview('30d')
		expect(overview.metrics.totalInstallations.value).toBe(2)
		expect(overview.metrics.dau.value).toBe(2)
		expect(overview.metrics.wau.value).toBe(2)
		expect(overview.metrics.mau.value).toBe(2)
		expect(overview.metrics.errorOccurrences.value).toBe(4)
		expect(JSON.stringify(overview)).not.toContain('installation_hash')
	})

	it('supports bound search, filters, sorting, and server pagination', async () => {
		const result = await api().errors(
			parseErrorQuery({
				range: '30d',
				search: 'render',
				platform: 'windows-fixture',
				page: '1',
				pageSize: '25',
				sort: 'occurrences',
				direction: 'desc',
			}),
		)
		expect(result.total).toBe(1)
		expect(result.items[0]).toMatchObject({
			fingerprint: 'fixture-render-01',
			occurrenceCount: 4,
			hasSample: true,
		})
		expect(JSON.stringify(result)).not.toMatch(/installation_hash|object_key/)
	})

	it('reads only a D1-registered sample and never returns internal keys', async () => {
		await env.ERROR_CONTEXTS.put(
			'errors/unregistered.json.gz',
			await gzip('{"message":"unregistered"}'),
		)
		const registered = await api().errorSample('fixture-render-01')
		expect(registered).toMatchObject({
			fingerprint: 'fixture-render-01',
			message: 'Fixture render failure',
		})
		expect(JSON.stringify(registered)).not.toMatch(/installation_hash|object_key|must-not-leak/)
		expect(await api().errorSample('fixture-unregistered')).toBeNull()
	})
})
