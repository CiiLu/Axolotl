import { Hono } from 'hono'

import { byteLength, redact, truncateUtf8 } from './redact'
import { batchSchema, type ErrorEvent, type TelemetryBatch } from './schema'

const MAX_REQUEST_BYTES = 64 * 1024
const MAX_CONTEXT_OBJECT_BYTES = 16 * 1024
const HARD_MAX_CONTEXTS_PER_DAY = 2_000
const HARD_MAX_SAMPLES_PER_GROUP = 3

export interface Bindings {
	DB: D1Database
	ERROR_CONTEXTS: R2Bucket
	INSTALLATION_HMAC_SECRET: string
	STORE_ERROR_CONTEXT?: string
	MAX_ERROR_CONTEXTS_PER_DAY?: string
	MAX_ERROR_SAMPLES_PER_GROUP?: string
}

type Variables = {
	requestId: string
}

const app = new Hono<{ Bindings: Bindings; Variables: Variables }>()

app.use('*', async (context, next) => {
	context.set('requestId', crypto.randomUUID())
	await next()
	context.header('Cache-Control', 'no-store')
})

app.get('/health', (context) =>
	context.json({ status: 'ok', schema_version: 1 }, 200, {
		'Cache-Control': 'no-store',
	}),
)

app.post('/v1/batch', async (context) => {
	const contentLength = Number(context.req.header('content-length') ?? '0')
	if (Number.isFinite(contentLength) && contentLength > MAX_REQUEST_BYTES) {
		return context.json({ error: 'request_too_large' }, 413)
	}

	const raw = await context.req.arrayBuffer()
	if (raw.byteLength > MAX_REQUEST_BYTES) {
		return context.json({ error: 'request_too_large' }, 413)
	}

	let input: unknown
	try {
		input = JSON.parse(new TextDecoder().decode(raw))
	} catch {
		return context.json({ error: 'invalid_json' }, 400)
	}

	const parsed = batchSchema.safeParse(input)
	if (!parsed.success) {
		return context.json(
			{
				error: 'invalid_batch',
				issues: parsed.error.issues.map((issue) => ({
					path: issue.path.join('.'),
					code: issue.code,
				})),
			},
			400,
		)
	}
	if (!context.env.INSTALLATION_HMAC_SECRET || context.env.INSTALLATION_HMAC_SECRET.length < 32) {
		return context.json({ error: 'service_unavailable' }, 503)
	}

	try {
		const accepted = await context.env.DB.prepare(
			'SELECT 1 FROM accepted_batches WHERE batch_id = ? LIMIT 1',
		)
			.bind(parsed.data.batch_id)
			.first()
		if (accepted) return context.json({ accepted: true, duplicate: true })

		const installationHash = await hmacInstallationId(
			context.env.INSTALLATION_HMAC_SECRET,
			parsed.data.installation_id,
		)
		const sanitized = sanitizeBatch(parsed.data)
		const objectKeys = await storeErrorContexts(context.env, sanitized)
		await persistBatch(context.env.DB, sanitized, installationHash, objectKeys)
		return context.json({ accepted: true, duplicate: false })
	} catch (error) {
		console.error('Telemetry ingestion failed', {
			requestId: context.get('requestId'),
			error: error instanceof Error ? error.message : String(error),
		})
		return context.json({ error: 'temporarily_unavailable' }, 503)
	}
})

app.notFound((context) => context.json({ error: 'not_found' }, 404))
app.onError((error, context) => {
	console.error('Unhandled telemetry worker error', {
		requestId: context.get('requestId'),
		error: error.message,
	})
	return context.json({ error: 'temporarily_unavailable' }, 503)
})

async function hmacInstallationId(secret: string, installationId: string): Promise<string> {
	const key = await crypto.subtle.importKey(
		'raw',
		new TextEncoder().encode(secret),
		{ name: 'HMAC', hash: 'SHA-256' },
		false,
		['sign'],
	)
	const signature = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(installationId))
	return [...new Uint8Array(signature)].map((byte) => byte.toString(16).padStart(2, '0')).join('')
}

function sanitizeBatch(batch: TelemetryBatch): TelemetryBatch {
	return {
		...batch,
		installation_id: batch.installation_id,
		app: {
			...batch.app,
			version: truncateUtf8(redact(batch.app.version), 64),
			platform: truncateUtf8(redact(batch.app.platform), 32),
			arch: truncateUtf8(redact(batch.app.arch), 32),
		},
		events: batch.events.map((event) => {
			if (event.type === 'heartbeat') return event
			return {
				...event,
				error_type: truncateUtf8(redact(event.error_type), 128),
				message: truncateUtf8(redact(event.message), 1_024),
				stack: event.stack ? truncateUtf8(redact(event.stack), 8_192) : event.stack,
				route: event.route ? truncateUtf8(redact(event.route), 256) : event.route,
				command: event.command ? truncateUtf8(redact(event.command), 256) : event.command,
				context: event.context
					? truncateUtf8(redact(event.context), MAX_CONTEXT_OBJECT_BYTES)
					: event.context,
			}
		}),
	}
}

async function storeErrorContexts(
	env: Bindings,
	batch: TelemetryBatch,
): Promise<Map<string, string>> {
	const objectKeys = new Map<string, string>()
	if (env.STORE_ERROR_CONTEXT !== 'true') return objectKeys

	const dailyLimit = Math.min(
		positiveInteger(env.MAX_ERROR_CONTEXTS_PER_DAY, HARD_MAX_CONTEXTS_PER_DAY),
		HARD_MAX_CONTEXTS_PER_DAY,
	)
	const sampleLimit = Math.min(
		positiveInteger(env.MAX_ERROR_SAMPLES_PER_GROUP, HARD_MAX_SAMPLES_PER_GROUP),
		HARD_MAX_SAMPLES_PER_GROUP,
	)
	const day = utcDay()

	for (const event of batch.events) {
		if (event.type !== 'error' || !event.context) continue
		const objectDay = eventDay(event)
		const objectKey = `errors/${objectDay}/${event.fingerprint}/${event.event_id}.json.gz`
		await env.DB.prepare(
			`INSERT OR IGNORE INTO error_context_reservations (
				event_id, day, fingerprint, app_version, object_key, created_at
			)
			SELECT ?, ?, ?, ?, ?, unixepoch()
			WHERE COALESCE((
				SELECT object_count FROM error_context_budget WHERE day = ?
			), 0) < ?
			AND COALESCE((
				SELECT sample_count FROM error_context_samples
				WHERE day = ? AND fingerprint = ? AND app_version = ?
			), 0) < ?`,
		)
			.bind(
				event.event_id,
				day,
				event.fingerprint,
				batch.app.version,
				objectKey,
				day,
				dailyLimit,
				day,
				event.fingerprint,
				batch.app.version,
				sampleLimit,
			)
			.run()

		const reservation = await env.DB.prepare(
			`SELECT r.object_key, b.object_count
			FROM error_context_reservations r
			JOIN error_context_budget b ON b.day = r.day
			WHERE r.event_id = ?`,
		)
			.bind(event.event_id)
			.first<{ object_key: string; object_count: number }>()
		if (!reservation) continue

		const serialized = buildContextObject(batch, event)
		const compressed = await gzip(serialized)
		await env.ERROR_CONTEXTS.put(reservation.object_key, compressed, {
			httpMetadata: { contentType: 'application/json', contentEncoding: 'gzip' },
			customMetadata: { schema_version: '1', day: objectDay },
		})
		objectKeys.set(event.event_id, reservation.object_key)
		warnAtThresholds(reservation.object_count, dailyLimit)
	}

	return objectKeys
}

function buildContextObject(batch: TelemetryBatch, event: ErrorEvent): string {
	const value = {
		schema_version: 1,
		event_id: event.event_id,
		occurred_at: event.occurred_at,
		fingerprint: event.fingerprint,
		app: batch.app,
		error_type: event.error_type,
		message: event.message,
		stack: event.stack,
		route: event.route,
		command: event.command,
		context: event.context,
	}
	let serialized = JSON.stringify(value)
	if (byteLength(serialized) <= MAX_CONTEXT_OBJECT_BYTES) return serialized

	const shrink = (field: 'context' | 'stack' | 'message' | 'route' | 'command' | 'error_type') => {
		const current = value[field] ?? ''
		const overflow = byteLength(serialized) - MAX_CONTEXT_OBJECT_BYTES
		value[field] = truncateUtf8(current, Math.max(0, byteLength(current) - overflow))
		serialized = JSON.stringify(value)
	}

	for (const field of ['context', 'stack', 'message', 'route', 'command', 'error_type'] as const) {
		if (byteLength(serialized) <= MAX_CONTEXT_OBJECT_BYTES) break
		shrink(field)
	}
	if (byteLength(serialized) > MAX_CONTEXT_OBJECT_BYTES) {
		return JSON.stringify({
			schema_version: 1,
			event_id: event.event_id,
			fingerprint: event.fingerprint,
		})
	}
	return serialized
}

async function gzip(input: string): Promise<ArrayBuffer> {
	const stream = new Blob([input]).stream().pipeThrough(new CompressionStream('gzip'))
	return await new Response(stream).arrayBuffer()
}

async function persistBatch(
	db: D1Database,
	batch: TelemetryBatch,
	installationHash: string,
	objectKeys: Map<string, string>,
): Promise<void> {
	const acceptedDay = utcDay()
	const statements: D1PreparedStatement[] = [
		db
			.prepare(
				`INSERT INTO installations (
					installation_hash, first_seen_at, last_seen_at,
					first_seen_day, app_version, platform, arch
				) VALUES (?, unixepoch(), unixepoch(), ?, ?, ?, ?)
				ON CONFLICT(installation_hash) DO UPDATE SET
					last_seen_at = excluded.last_seen_at,
					app_version = excluded.app_version,
					platform = excluded.platform,
					arch = excluded.arch`,
			)
			.bind(installationHash, acceptedDay, batch.app.version, batch.app.platform, batch.app.arch),
	]

	for (const event of batch.events) {
		const day = eventDay(event)
		if (event.type === 'heartbeat') {
			statements.push(
				db
					.prepare(
						'INSERT OR IGNORE INTO daily_active (day, installation_hash, app_version, platform, arch) VALUES (?, ?, ?, ?, ?)',
					)
					.bind(day, installationHash, batch.app.version, batch.app.platform, batch.app.arch),
			)
			continue
		}

		statements.push(
			db
				.prepare(
					`INSERT OR IGNORE INTO error_reports (
						event_id, installation_hash, day, occurred_at, fingerprint,
						app_version, platform, arch, error_type, message,
						occurrence_count, object_key, created_at
					) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())`,
				)
				.bind(
					event.event_id,
					installationHash,
					day,
					event.occurred_at,
					event.fingerprint,
					batch.app.version,
					batch.app.platform,
					batch.app.arch,
					event.error_type,
					event.message,
					event.occurrence_count,
					objectKeys.get(event.event_id) ?? null,
				),
		)
	}

	statements.push(
		db
			.prepare(
				'INSERT INTO accepted_batches (batch_id, installation_hash, accepted_at) VALUES (?, ?, unixepoch())',
			)
			.bind(batch.batch_id, installationHash),
	)
	await db.batch(statements)
}

async function runMaintenance(db: D1Database): Promise<void> {
	const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1_000).toISOString().slice(0, 10)
	await db.batch([
		db
			.prepare(
				`INSERT INTO daily_totals (
					day, new_installations, active_installations, error_occurrences, distinct_error_groups
				) VALUES (
					?,
					(SELECT COUNT(*) FROM installations WHERE first_seen_day = ?),
					(SELECT COUNT(*) FROM daily_active WHERE day = ?),
					(SELECT COALESCE(SUM(occurrence_count), 0) FROM error_reports WHERE day = ?),
					(SELECT COUNT(*) FROM error_daily WHERE day = ?)
				)
				ON CONFLICT(day) DO UPDATE SET
					new_installations = excluded.new_installations,
					active_installations = excluded.active_installations,
					error_occurrences = excluded.error_occurrences,
					distinct_error_groups = excluded.distinct_error_groups`,
			)
			.bind(yesterday, yesterday, yesterday, yesterday, yesterday),
		db.prepare("DELETE FROM daily_active WHERE day < date('now', '-35 days')"),
		db.prepare("DELETE FROM error_reports WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_reservations WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_samples WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_context_budget WHERE day < date('now', '-30 days')"),
		db.prepare("DELETE FROM error_daily WHERE day < date('now', '-365 days')"),
		db.prepare("DELETE FROM error_groups WHERE last_seen_day < date('now', '-365 days')"),
		db.prepare("DELETE FROM accepted_batches WHERE accepted_at < unixepoch('now', '-8 days')"),
	])
}

function positiveInteger(value: string | undefined, fallback: number): number {
	const parsed = Number(value)
	return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback
}

function utcDay(): string {
	return new Date().toISOString().slice(0, 10)
}

function eventDay(event: TelemetryBatch['events'][number]): string {
	if (event.type === 'heartbeat') return event.day
	return new Date(event.occurred_at).toISOString().slice(0, 10)
}

function warnAtThresholds(count: number, limit: number): void {
	const thresholds = [0.5, 0.75, 0.9].map((ratio) => Math.ceil(limit * ratio))
	if (thresholds.includes(count)) {
		console.warn('R2 telemetry context budget threshold reached', { count, limit })
	}
}

export { app, hmacInstallationId, runMaintenance, sanitizeBatch }

export default {
	fetch: app.fetch,
	async scheduled(_controller: ScheduledController, env: Bindings, context: ExecutionContext) {
		context.waitUntil(runMaintenance(env.DB))
	},
} satisfies ExportedHandler<Bindings>
