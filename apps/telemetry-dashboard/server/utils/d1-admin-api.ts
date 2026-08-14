import type {
	ActivityDto,
	AdminRange,
	DistributionItemDto,
	DistributionsDto,
	ErrorDetailDto,
	ErrorFiltersDto,
	ErrorRowDto,
	ErrorSampleDto,
	ErrorsPageDto,
	OverviewDto,
	ServiceCheckDto,
	SystemDto,
} from '../../shared/types/telemetry'
import type { TelemetryAdminApi } from './admin-api'
import { unavailable } from './errors'
import { type ErrorQuery, rangeDays, startDay } from './validation'

interface CountRow {
	count: number
}

interface OverviewRow {
	total_installations: number
	dau: number
	wau: number
	mau: number
	new_today: number
	error_occurrences: number
	distinct_groups: number
	r2_today: number
}

interface ActivityRow {
	day: string
	active_installations: number
	new_installations: number
	error_occurrences: number
}

interface DistributionRow {
	label: string
	value: number
}

interface ErrorRow {
	fingerprint: string
	error_type: string
	latest_message: string
	app_version: string
	first_seen: string
	last_seen: string
	occurrence_count: number
	affected_installations: number
	has_sample: number
}

interface SampleRegistration {
	object_key: string
}

export interface TelemetryQueryResult<T = Record<string, unknown>> {
	results: T[]
}

export interface TelemetryStatement {
	bind(...values: unknown[]): TelemetryStatement
	first<T = Record<string, unknown>>(): Promise<T | null>
	all<T = Record<string, unknown>>(): Promise<TelemetryQueryResult<T>>
}

export interface TelemetryDatabase {
	prepare(sql: string): TelemetryStatement
	batch(statements: TelemetryStatement[]): Promise<TelemetryQueryResult[]>
}

export interface TelemetryObject {
	body: ReadableStream
	httpMetadata?: { contentEncoding?: string }
}

export interface TelemetryObjectStore {
	get(
		key: string,
		options?: { range?: { offset: number; length: number } },
	): Promise<TelemetryObject | null>
}

const SAMPLE_UNCOMPRESSED_LIMIT = 32 * 1024

function utcDay(now = new Date()): string {
	return now.toISOString().slice(0, 10)
}

function mapError(row: ErrorRow): ErrorRowDto {
	return {
		fingerprint: row.fingerprint,
		errorType: row.error_type,
		latestMessage: row.latest_message,
		appVersion: row.app_version,
		firstSeen: row.first_seen,
		lastSeen: row.last_seen,
		occurrenceCount: Number(row.occurrence_count),
		affectedInstallations: Number(row.affected_installations),
		hasSample: Boolean(row.has_sample),
	}
}

function fillActivity(range: AdminRange, rows: ActivityRow[], now = new Date()): ActivityDto {
	const values = new Map(rows.map((row) => [row.day, row]))
	const start = new Date(`${startDay(range, now)}T00:00:00.000Z`)
	const points = Array.from({ length: rangeDays(range) }, (_, index) => {
		const day = new Date(start)
		day.setUTCDate(start.getUTCDate() + index)
		const key = day.toISOString().slice(0, 10)
		const row = values.get(key)
		return {
			day: key,
			activeInstallations: Number(row?.active_installations ?? 0),
			newInstallations: Number(row?.new_installations ?? 0),
			errorOccurrences: Number(row?.error_occurrences ?? 0),
		}
	})
	return { range, points }
}

function service(
	status: ServiceCheckDto['status'],
	label: string,
	detail: string,
): ServiceCheckDto {
	return { status, label, detail }
}

export class D1TelemetryAdminApi implements TelemetryAdminApi {
	constructor(
		private readonly db: TelemetryDatabase,
		private readonly r2: TelemetryObjectStore | undefined,
		private readonly options: {
			storeErrorContext: boolean
			healthUrl: string
			fetcher?: typeof fetch
			now?: () => Date
		},
	) {}

	private now(): Date {
		return this.options.now?.() ?? new Date()
	}

	async overview(range: AdminRange): Promise<OverviewDto> {
		const today = utcDay(this.now())
		const start = startDay(range, this.now())
		const row = await this.db
			.prepare(
				`SELECT
					(SELECT COUNT(*) FROM installations) AS total_installations,
					(SELECT COUNT(*) FROM daily_active WHERE day = ?) AS dau,
					(SELECT COUNT(DISTINCT installation_hash) FROM daily_active WHERE day >= date(?, '-6 days') AND day <= ?) AS wau,
					(SELECT COUNT(DISTINCT installation_hash) FROM daily_active WHERE day >= date(?, '-29 days') AND day <= ?) AS mau,
					COALESCE((SELECT new_installations FROM daily_totals WHERE day = ?), 0) AS new_today,
					COALESCE((SELECT SUM(error_occurrences) FROM daily_totals WHERE day >= ? AND day <= ?), 0) AS error_occurrences,
					(SELECT COUNT(DISTINCT fingerprint) FROM error_daily WHERE day >= ? AND day <= ?) AS distinct_groups,
					COALESCE((SELECT object_count FROM error_context_budget WHERE day = ?), 0) AS r2_today`,
			)
			.bind(today, today, today, today, today, today, start, today, start, today, today)
			.first<OverviewRow>()
		if (!row) throw unavailable()
		const metric = (value: number, label: string) => ({ value: Number(value), label })
		return {
			range,
			generatedAt: this.now().toISOString(),
			metrics: {
				totalInstallations: metric(row.total_installations, '历史累计主动同意遥测的安装'),
				dau: metric(row.dau, '今日 UTC 唯一活跃安装'),
				wau: metric(row.wau, '最近 7 天唯一活跃安装'),
				mau: metric(row.mau, '最近 30 天唯一活跃安装'),
				newInstallationsToday: metric(row.new_today, '今日 UTC 首次出现'),
				errorOccurrences: metric(row.error_occurrences, `${range} 范围内发生次数`),
				distinctErrorGroups: metric(row.distinct_groups, `${range} 范围内错误指纹`),
				r2SamplesToday: metric(row.r2_today, '今日 UTC 已存储样本'),
			},
		}
	}

	async activity(range: AdminRange): Promise<ActivityDto> {
		const rows = await this.db
			.prepare(
				`SELECT day, active_installations, new_installations, error_occurrences
				FROM daily_totals WHERE day >= ? AND day <= ? ORDER BY day ASC`,
			)
			.bind(startDay(range, this.now()), utcDay(this.now()))
			.all<ActivityRow>()
		return fillActivity(range, rows.results, this.now())
	}

	async distributions(range: AdminRange): Promise<DistributionsDto> {
		const start = startDay(range, this.now())
		const end = utcDay(this.now())
		const query = async (
			field: 'app_version' | 'platform' | 'arch',
		): Promise<DistributionItemDto[]> => {
			const result = await this.db
				.prepare(
					`SELECT ${field} AS label, COUNT(DISTINCT installation_hash) AS value
					FROM daily_active WHERE day >= ? AND day <= ?
					GROUP BY ${field} ORDER BY value DESC, label ASC LIMIT 12`,
				)
				.bind(start, end)
				.all<DistributionRow>()
			return result.results.map((row) => ({ label: row.label, value: Number(row.value) }))
		}
		const [versions, platforms, architectures] = await Promise.all([
			query('app_version'),
			query('platform'),
			query('arch'),
		])
		return { range, versions, platforms, architectures }
	}

	async errors(query: ErrorQuery): Promise<ErrorsPageDto> {
		const conditions = ['ed.day >= ?', 'ed.day <= ?']
		const bindings: unknown[] = [startDay(query.range, this.now()), utcDay(this.now())]
		if (query.search) {
			const escaped = query.search.toLowerCase().replace(/[\\%_]/g, '\\$&')
			conditions.push(
				"(LOWER(ed.fingerprint) LIKE ? ESCAPE '\\' OR LOWER(eg.latest_message) LIKE ? ESCAPE '\\' OR LOWER(eg.latest_error_type) LIKE ? ESCAPE '\\')",
			)
			bindings.push(`%${escaped}%`, `%${escaped}%`, `%${escaped}%`)
		}
		if (query.version) {
			conditions.push('ed.app_version = ?')
			bindings.push(query.version)
		}
		if (query.errorType) {
			conditions.push('eg.latest_error_type = ?')
			bindings.push(query.errorType)
		}
		if (query.platform) {
			conditions.push(
				'EXISTS (SELECT 1 FROM error_reports er WHERE er.fingerprint = ed.fingerprint AND er.app_version = ed.app_version AND er.platform = ? AND er.day >= ? AND er.day <= ?)',
			)
			bindings.push(query.platform, startDay(query.range, this.now()), utcDay(this.now()))
		}
		if (query.hasSample !== null) {
			conditions.push(
				query.hasSample ? 'eg.sample_object_key IS NOT NULL' : 'eg.sample_object_key IS NULL',
			)
		}
		const where = conditions.join(' AND ')
		const scoped = `SELECT
			ed.fingerprint,
			MIN(ed.day) AS first_seen,
			MAX(ed.day) AS last_seen,
			SUM(ed.occurrence_count) AS occurrence_count,
			SUM(ed.installation_count) AS affected_installations,
			MAX(CASE WHEN eg.sample_object_key IS NOT NULL THEN 1 ELSE 0 END) AS has_sample
		FROM error_daily ed
		JOIN error_groups eg ON eg.fingerprint = ed.fingerprint AND eg.app_version = ed.app_version
		WHERE ${where}
		GROUP BY ed.fingerprint`
		const totalRow = await this.db
			.prepare(`SELECT COUNT(*) AS count FROM (${scoped})`)
			.bind(...bindings)
			.first<CountRow>()
		const sortColumns: Record<ErrorQuery['sort'], string> = {
			lastSeen: 's.last_seen',
			firstSeen: 's.first_seen',
			occurrences: 's.occurrence_count',
			installations: 's.affected_installations',
		}
		const direction = query.direction === 'asc' ? 'ASC' : 'DESC'
		const offset = (query.page - 1) * query.pageSize
		const rows = await this.db
			.prepare(
				`WITH scoped AS (${scoped})
				SELECT
					s.*,
					COALESCE((SELECT latest_error_type FROM error_groups WHERE fingerprint = s.fingerprint ORDER BY last_seen_day DESC LIMIT 1), 'Unknown') AS error_type,
					COALESCE((SELECT latest_message FROM error_groups WHERE fingerprint = s.fingerprint ORDER BY last_seen_day DESC LIMIT 1), '') AS latest_message,
					COALESCE((SELECT app_version FROM error_groups WHERE fingerprint = s.fingerprint ORDER BY last_seen_day DESC LIMIT 1), 'Unknown') AS app_version
				FROM scoped s
				ORDER BY ${sortColumns[query.sort]} ${direction}, s.fingerprint ASC
				LIMIT ? OFFSET ?`,
			)
			.bind(...bindings, query.pageSize, offset)
			.all<ErrorRow>()
		const filters = await this.errorFilters()
		const total = Number(totalRow?.count ?? 0)
		return {
			items: rows.results.map(mapError),
			page: query.page,
			pageSize: query.pageSize,
			total,
			totalPages: Math.max(1, Math.ceil(total / query.pageSize)),
			filters,
		}
	}

	private async errorFilters(): Promise<ErrorFiltersDto> {
		const [versions, platforms, errorTypes] = await Promise.all([
			this.db
				.prepare(
					'SELECT DISTINCT app_version AS value FROM error_groups ORDER BY value DESC LIMIT 100',
				)
				.all<{ value: string }>(),
			this.db
				.prepare(
					'SELECT DISTINCT platform AS value FROM error_reports ORDER BY value ASC LIMIT 100',
				)
				.all<{ value: string }>(),
			this.db
				.prepare(
					'SELECT DISTINCT latest_error_type AS value FROM error_groups ORDER BY value ASC LIMIT 100',
				)
				.all<{ value: string }>(),
		])
		return {
			versions: versions.results.map((row) => row.value),
			platforms: platforms.results.map((row) => row.value),
			errorTypes: errorTypes.results.map((row) => row.value),
		}
	}

	async errorDetail(fingerprint: string): Promise<ErrorDetailDto | null> {
		const row = await this.db
			.prepare(
				`SELECT fingerprint, latest_error_type AS error_type, latest_message,
					app_version, first_seen_day AS first_seen, last_seen_day AS last_seen,
					occurrence_count, installation_count AS affected_installations,
					CASE WHEN sample_object_key IS NULL THEN 0 ELSE 1 END AS has_sample
				FROM error_groups WHERE fingerprint = ? ORDER BY last_seen_day DESC LIMIT 1`,
			)
			.bind(fingerprint)
			.first<ErrorRow>()
		if (!row) return null
		return { ...mapError(row), route: null, command: null, stack: null }
	}

	async errorSample(fingerprint: string): Promise<ErrorSampleDto | null> {
		if (!this.r2) return null
		const registration = await this.db
			.prepare(
				`SELECT object_key FROM error_reports
				WHERE fingerprint = ? AND object_key IS NOT NULL
				ORDER BY occurred_at DESC LIMIT 1`,
			)
			.bind(fingerprint)
			.first<SampleRegistration>()
		if (!registration) return null
		const object = await this.r2.get(registration.object_key)
		if (!object?.body) return null
		let stream: ReadableStream = object.body
		if (
			object.httpMetadata?.contentEncoding === 'gzip' ||
			registration.object_key.endsWith('.gz')
		) {
			stream = stream.pipeThrough(new DecompressionStream('gzip'))
		}
		const bytes = await new Response(stream).arrayBuffer()
		if (bytes.byteLength > SAMPLE_UNCOMPRESSED_LIMIT) throw unavailable()
		let value: Record<string, unknown>
		try {
			value = JSON.parse(new TextDecoder().decode(bytes)) as Record<string, unknown>
		} catch {
			throw unavailable()
		}
		const app =
			typeof value.app === 'object' && value.app ? (value.app as Record<string, unknown>) : {}
		const text = (input: unknown, limit: number): string =>
			typeof input === 'string' ? input.slice(0, limit) : ''
		const optional = (input: unknown, limit: number): string | null => text(input, limit) || null
		return {
			fingerprint,
			occurredAt: text(value.occurred_at, 64),
			appVersion: text(app.version, 64),
			platform: text(app.platform, 32),
			architecture: text(app.arch, 32),
			errorType: text(value.error_type, 128),
			message: text(value.message, 1_024),
			stack: optional(value.stack, 8_192),
			route: optional(value.route, 256),
			command: optional(value.command, 256),
			context: optional(value.context, 16_384),
		}
	}

	async system(): Promise<SystemDto> {
		const now = this.now()
		let publicWorker = service('unavailable', '不可用', '健康检查端点未响应')
		try {
			const response = await (this.options.fetcher ?? fetch)(this.options.healthUrl, {
				signal: AbortSignal.timeout(3_000),
				headers: { accept: 'application/json' },
			})
			if (response.ok) publicWorker = service('available', '运行正常', '公开采集服务健康检查通过')
		} catch {
			publicWorker = service('unavailable', '不可用', '健康检查端点未响应')
		}

		let d1 = service('unavailable', '不可用', 'D1 查询失败')
		let latestDataDay: string | null = null
		let budget = 0
		let sampleKey: string | null = null
		try {
			const result = await this.db.batch([
				this.db.prepare('SELECT MAX(day) AS value FROM daily_totals'),
				this.db
					.prepare(
						'SELECT COALESCE(object_count, 0) AS value FROM error_context_budget WHERE day = ?',
					)
					.bind(utcDay(now)),
				this.db.prepare(
					'SELECT object_key AS value FROM error_reports WHERE object_key IS NOT NULL ORDER BY occurred_at DESC LIMIT 1',
				),
			])
			latestDataDay = (result[0].results[0] as { value?: string | null } | undefined)?.value ?? null
			budget = Number((result[1].results[0] as { value?: number } | undefined)?.value ?? 0)
			sampleKey = (result[2].results[0] as { value?: string | null } | undefined)?.value ?? null
			d1 = service('available', '可查询', '只读遥测查询可用')
		} catch {
			d1 = service('unavailable', '不可用', 'D1 查询失败')
		}

		let r2 = service('unavailable', '不可用', '没有可读取的登记样本')
		if (!this.options.storeErrorContext) {
			r2 = service('degraded', '已停用', '错误上下文存储已停用')
		} else if (this.r2 && sampleKey) {
			try {
				const sample = await this.r2.get(sampleKey, { range: { offset: 0, length: 1 } })
				if (sample) r2 = service('available', '可读取', '已确认一个登记样本可读取')
			} catch {
				r2 = service('unavailable', '不可用', '登记样本无法读取')
			}
		} else if (this.r2) {
			r2 = service('degraded', '暂无样本', 'R2 binding 正常，但当前没有登记样本')
		}

		const yesterday = new Date(now)
		yesterday.setUTCDate(yesterday.getUTCDate() - 1)
		const expected = yesterday.toISOString().slice(0, 10)
		const cron = !latestDataDay
			? service('unavailable', '不可用', '没有可用的聚合日期')
			: latestDataDay >= expected
				? service('available', '数据最新', `最近聚合日期：${latestDataDay}（UTC）`)
				: service('degraded', '数据滞后', `最近聚合日期：${latestDataDay}（UTC）`)

		return {
			generatedAt: now.toISOString(),
			publicWorker,
			d1,
			r2,
			storeErrorContext: this.options.storeErrorContext,
			r2Budget: { used: budget, limit: 2000 },
			limits: {
				samplesPerGroup: 3,
				dailyActiveRetentionDays: 35,
				errorReportsRetentionDays: 30,
				r2RetentionDays: 30,
				errorAggregatesRetentionDays: 365,
			},
			latestDataDay,
			cron,
			accountUsage: service('unavailable', '不可用', '尚未配置 Cloudflare Analytics API'),
		}
	}
}

export { fillActivity }
