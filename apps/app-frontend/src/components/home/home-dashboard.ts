export const HOME_DASHBOARD_VERSION = 1 as const
export const HOME_WIDGET_SIZES = ['1x1', '2x1', '1x2', '2x2'] as const
export const HOME_RECENT_LIMIT_OPTIONS = [2, 4, 6, 8] as const
export const HOME_RECENT_DEFAULT_LIMIT = 4
export const HOME_GREETING_MODES = ['greeting', 'text-and-greeting', 'text'] as const
export const HOME_GREETING_DEFAULT_MODE = 'greeting'
export const HOME_GREETING_FONTS = ['sans', 'minecraft', 'mono', 'serif'] as const
export const HOME_GREETING_DEFAULT_FONT = 'sans'
export const HOME_GREETING_FONT_SIZE_MIN = 16
export const HOME_GREETING_FONT_SIZE_MAX = 32
export const HOME_GREETING_DEFAULT_FONT_SIZE = 22
export const HOME_DASHBOARD_PRESET_IDS = ['balanced'] as const

export type HomeWidgetSize = (typeof HOME_WIDGET_SIZES)[number]
export type HomeRecentLimit = (typeof HOME_RECENT_LIMIT_OPTIONS)[number]
export type HomeGreetingMode = (typeof HOME_GREETING_MODES)[number]
export type HomeGreetingFont = (typeof HOME_GREETING_FONTS)[number]
export type HomeDashboardPresetId = (typeof HOME_DASHBOARD_PRESET_IDS)[number]
export type HomeWidgetKind =
	| 'greeting'
	| 'recent'
	| 'calendar'
	| 'pinned-instances'
	| 'pinned-worlds'
	| 'pinned-servers'
	| 'instance'
	| 'world'
	| 'server'

export type HomeWidgetTarget = {
	instanceId: string
	path?: string
	address?: string
	fallbackLabel: string
}

export type HomeWidgetOptions = {
	recentLimit?: HomeRecentLimit
	greetingMode?: HomeGreetingMode
	greetingText?: string
	greetingFont?: HomeGreetingFont
	greetingFontSize?: number
}

export type HomeWidgetPlacement = {
	id: string
	kind: HomeWidgetKind
	size: HomeWidgetSize
	target?: HomeWidgetTarget
	options?: HomeWidgetOptions
}

export type HomeDashboardConfig = {
	version: typeof HOME_DASHBOARD_VERSION
	widgets: HomeWidgetPlacement[]
}

export type PackedHomeWidget = HomeWidgetPlacement & {
	column: number
	row: number
	effectiveColumns: number
	effectiveRows: number
}

export type HomeDashboardSaveQueue = {
	enqueue: (config: HomeDashboardConfig, rollback: HomeDashboardConfig) => Promise<void>
	flush: () => Promise<void>
}

export const HOME_WIDGET_SIZE_OPTIONS: Record<HomeWidgetKind, readonly HomeWidgetSize[]> = {
	greeting: ['2x1'],
	recent: ['2x1', '1x2', '2x2'],
	calendar: ['1x2'],
	'pinned-instances': HOME_WIDGET_SIZES,
	'pinned-worlds': HOME_WIDGET_SIZES,
	'pinned-servers': HOME_WIDGET_SIZES,
	instance: ['1x1', '2x1'],
	world: ['1x1', '2x1'],
	server: ['1x1', '2x1'],
}

export const HOME_WIDGET_DEFAULT_SIZE: Record<HomeWidgetKind, HomeWidgetSize> = {
	greeting: '2x1',
	recent: '2x2',
	calendar: '1x2',
	'pinned-instances': '2x2',
	'pinned-worlds': '1x2',
	'pinned-servers': '1x2',
	instance: '1x1',
	world: '1x1',
	server: '1x1',
}

const HOME_WIDGET_KINDS = new Set<HomeWidgetKind>(
	Object.keys(HOME_WIDGET_DEFAULT_SIZE) as HomeWidgetKind[],
)

function createPlacement(
	kind: HomeWidgetKind,
	size = HOME_WIDGET_DEFAULT_SIZE[kind],
): HomeWidgetPlacement {
	return {
		id: crypto.randomUUID(),
		kind,
		size,
		...(kind === 'recent' ? { options: { recentLimit: HOME_RECENT_DEFAULT_LIMIT } } : {}),
		...(kind === 'greeting'
			? {
					options: {
						greetingMode: HOME_GREETING_DEFAULT_MODE,
						greetingFont: HOME_GREETING_DEFAULT_FONT,
						greetingFontSize: HOME_GREETING_DEFAULT_FONT_SIZE,
					},
				}
			: {}),
	}
}

export function createDefaultHomeDashboard(includeRecent = true): HomeDashboardConfig {
	return {
		version: HOME_DASHBOARD_VERSION,
		widgets: [
			createPlacement('greeting'),
			...(includeRecent ? [createPlacement('recent')] : []),
			createPlacement('calendar'),
			createPlacement('pinned-instances'),
			createPlacement('pinned-worlds'),
			createPlacement('pinned-servers'),
		],
	}
}

export function createHomeDashboardPreset(id: HomeDashboardPresetId): HomeDashboardConfig {
	switch (id) {
		case 'balanced':
			return {
				version: HOME_DASHBOARD_VERSION,
				widgets: [
					createPlacement('greeting', '2x1'),
					createPlacement('calendar', '1x2'),
					createPlacement('recent', '2x2'),
					createPlacement('pinned-worlds', '1x2'),
					createPlacement('pinned-servers', '2x1'),
					createPlacement('pinned-instances', '2x2'),
				],
			}
	}
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function normalizeTarget(value: unknown): HomeWidgetTarget | undefined {
	if (!isRecord(value) || typeof value.instanceId !== 'string' || !value.instanceId)
		return undefined
	if (typeof value.fallbackLabel !== 'string' || !value.fallbackLabel) return undefined

	return {
		instanceId: value.instanceId,
		...(typeof value.path === 'string' ? { path: value.path } : {}),
		...(typeof value.address === 'string' ? { address: value.address } : {}),
		fallbackLabel: value.fallbackLabel,
	}
}

function normalizeOptions(kind: HomeWidgetKind, value: unknown): HomeWidgetOptions | undefined {
	if (kind === 'recent') {
		const recentLimit =
			isRecord(value) && HOME_RECENT_LIMIT_OPTIONS.includes(value.recentLimit as HomeRecentLimit)
				? (value.recentLimit as HomeRecentLimit)
				: HOME_RECENT_DEFAULT_LIMIT
		return { recentLimit }
	}

	if (kind === 'greeting') {
		const greetingMode =
			isRecord(value) && HOME_GREETING_MODES.includes(value.greetingMode as HomeGreetingMode)
				? (value.greetingMode as HomeGreetingMode)
				: HOME_GREETING_DEFAULT_MODE
		const greetingText =
			isRecord(value) && typeof value.greetingText === 'string'
				? value.greetingText.trim().slice(0, 120)
				: ''
		const greetingFont =
			isRecord(value) && HOME_GREETING_FONTS.includes(value.greetingFont as HomeGreetingFont)
				? (value.greetingFont as HomeGreetingFont)
				: HOME_GREETING_DEFAULT_FONT
		const greetingFontSize = normalizeGreetingFontSize(
			isRecord(value) ? value.greetingFontSize : undefined,
		)
		return {
			greetingMode,
			...(greetingText ? { greetingText } : {}),
			greetingFont,
			greetingFontSize,
		}
	}

	return undefined
}

function normalizeGreetingFontSize(value: unknown): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) {
		return HOME_GREETING_DEFAULT_FONT_SIZE
	}

	return Math.min(
		HOME_GREETING_FONT_SIZE_MAX,
		Math.max(HOME_GREETING_FONT_SIZE_MIN, Math.round(value)),
	)
}

export function normalizeHomeDashboard(value: unknown): HomeDashboardConfig | null {
	if (
		!isRecord(value) ||
		value.version !== HOME_DASHBOARD_VERSION ||
		!Array.isArray(value.widgets)
	) {
		return null
	}

	const usedIds = new Set<string>()
	const widgets = value.widgets.flatMap((candidate): HomeWidgetPlacement[] => {
		if (!isRecord(candidate) || typeof candidate.kind !== 'string') return []
		if (!HOME_WIDGET_KINDS.has(candidate.kind as HomeWidgetKind)) return []

		const kind = candidate.kind as HomeWidgetKind
		const target = normalizeTarget(candidate.target)
		const options = normalizeOptions(kind, candidate.options)
		if ((kind === 'instance' || kind === 'world' || kind === 'server') && !target) return []
		if (kind === 'world' && !target?.path) return []
		if (kind === 'server' && !target?.address) return []

		let id = typeof candidate.id === 'string' && candidate.id ? candidate.id : crypto.randomUUID()
		if (usedIds.has(id)) id = crypto.randomUUID()
		usedIds.add(id)

		const requestedSize = typeof candidate.size === 'string' ? candidate.size : ''
		const size = HOME_WIDGET_SIZE_OPTIONS[kind].includes(requestedSize as HomeWidgetSize)
			? (requestedSize as HomeWidgetSize)
			: HOME_WIDGET_DEFAULT_SIZE[kind]

		return [
			{
				id,
				kind,
				size,
				...(target ? { target } : {}),
				...(options ? { options } : {}),
			},
		]
	})

	return { version: HOME_DASHBOARD_VERSION, widgets }
}

export function replaceHomeDashboardWidgets(
	config: HomeDashboardConfig,
	widgets: HomeWidgetPlacement[],
): HomeDashboardConfig {
	return { ...config, widgets }
}

export function addHomeWidget(
	config: HomeDashboardConfig,
	widget: HomeWidgetPlacement,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(config, [...config.widgets, widget])
}

export function removeHomeWidget(config: HomeDashboardConfig, id: string): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.filter((widget) => widget.id !== id),
	)
}

export function resizeHomeWidget(
	config: HomeDashboardConfig,
	id: string,
	size: HomeWidgetSize,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && HOME_WIDGET_SIZE_OPTIONS[widget.kind].includes(size)
				? { ...widget, size }
				: widget,
		),
	)
}

export function setHomeRecentLimit(
	config: HomeDashboardConfig,
	id: string,
	recentLimit: HomeRecentLimit,
): HomeDashboardConfig {
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && widget.kind === 'recent'
				? { ...widget, options: { ...widget.options, recentLimit } }
				: widget,
		),
	)
}

export function setHomeGreetingOptions(
	config: HomeDashboardConfig,
	id: string,
	greetingMode: HomeGreetingMode,
	greetingText: string,
	greetingFont: HomeGreetingFont,
	greetingFontSize: number,
): HomeDashboardConfig {
	const normalizedText = greetingText.trim().slice(0, 120)
	const normalizedFont = HOME_GREETING_FONTS.includes(greetingFont)
		? greetingFont
		: HOME_GREETING_DEFAULT_FONT
	return replaceHomeDashboardWidgets(
		config,
		config.widgets.map((widget) =>
			widget.id === id && widget.kind === 'greeting'
				? {
						...widget,
						options: {
							greetingMode,
							...(normalizedText ? { greetingText: normalizedText } : {}),
							greetingFont: normalizedFont,
							greetingFontSize: normalizeGreetingFontSize(greetingFontSize),
						},
					}
				: widget,
		),
	)
}

export function moveHomeWidget(
	config: HomeDashboardConfig,
	index: number,
	direction: -1 | 1,
): HomeDashboardConfig {
	const target = index + direction
	if (
		index < 0 ||
		index >= config.widgets.length ||
		target < 0 ||
		target >= config.widgets.length
	) {
		return config
	}

	const widgets = [...config.widgets]
	const [widget] = widgets.splice(index, 1)
	widgets.splice(target, 0, widget)
	return replaceHomeDashboardWidgets(config, widgets)
}

export function createHomeDashboardSaveQueue(
	persist: (config: HomeDashboardConfig) => Promise<void>,
	onRollback: (config: HomeDashboardConfig) => void,
	onError: (error: unknown) => void,
): HomeDashboardSaveQueue {
	let queue = Promise.resolve()
	let version = 0

	return {
		enqueue(config, rollback) {
			const operationVersion = ++version
			queue = queue.then(async () => {
				try {
					await persist(config)
				} catch (error) {
					if (operationVersion === version) onRollback(rollback)
					onError(error)
				}
			})
			return queue
		},
		flush: () => queue,
	}
}

export function getHomeGridColumnCount(width: number): number {
	const minimumColumnWidth = 240
	const gap = 16
	return Math.min(
		4,
		Math.max(1, Math.floor((Math.max(0, width) + gap) / (minimumColumnWidth + gap))),
	)
}

export function getHomeWidgetSpan(size: HomeWidgetSize, columnCount: number) {
	const [columns, rows] = size.split('x').map(Number)
	return {
		columns: Math.min(columns, Math.max(1, columnCount)),
		rows,
	}
}

export function packHomeWidgets(
	widgets: readonly HomeWidgetPlacement[],
	columnCount: number,
): PackedHomeWidget[] {
	const columns = Math.max(1, columnCount)
	const occupied: boolean[][] = []

	const fits = (column: number, row: number, width: number, height: number) => {
		if (column + width > columns) return false
		for (let y = row; y < row + height; y += 1) {
			for (let x = column; x < column + width; x += 1) {
				if (occupied[y]?.[x]) return false
			}
		}
		return true
	}

	return widgets.map((widget) => {
		const span = getHomeWidgetSpan(widget.size, columns)
		let row = 0
		let column = 0
		while (!fits(column, row, span.columns, span.rows)) {
			column += 1
			if (column >= columns) {
				column = 0
				row += 1
			}
		}

		for (let y = row; y < row + span.rows; y += 1) {
			occupied[y] ??= Array.from({ length: columns }, () => false)
			for (let x = column; x < column + span.columns; x += 1) occupied[y][x] = true
		}

		return {
			...widget,
			column: column + 1,
			row: row + 1,
			effectiveColumns: span.columns,
			effectiveRows: span.rows,
		}
	})
}
