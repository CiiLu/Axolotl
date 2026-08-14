import type { ComputedRef, Ref } from 'vue'
import { computed, ref, watch } from 'vue'

import { defineMessages, useVIntl } from '#ui/composables/i18n'
import {
	commonProjectTypeCategoryMessages,
	normalizeProjectType,
} from '#ui/utils/common-messages'

import type { ContentItem } from '../types'

export interface MetadataFilterOption {
	value: string
	label: string
}

export interface MetadataFilterCategory {
	key: string
	label: string
	searchable?: boolean
	options: MetadataFilterOption[]
}

interface MetadataFilterDefinition {
	key: string
	label: string
	searchable?: boolean
	values: (item: ContentItem) => string[]
	labelFor: (value: string) => string
	/** Preferred option order; unlisted values (including 未知) sort after ordered ones. */
	order?: string[]
}

const UNKNOWN = 'unknown'

const openSourceLicenseIds = new Set([
	'0BSD',
	'AFL-3.0',
	'AGPL-3.0',
	'Apache-2.0',
	'Artistic-2.0',
	'BSD-2-Clause',
	'BSD-3-Clause',
	'BSL-1.0',
	'CDDL-1.0',
	'ECL-2.0',
	'EPL-1.0',
	'EPL-2.0',
	'EUPL-1.1',
	'EUPL-1.2',
	'GPL-2.0',
	'GPL-3.0',
	'ISC',
	'LGPL-2.1',
	'LGPL-3.0',
	'MIT',
	'MPL-2.0',
	'NCSA',
	'OSL-3.0',
	'PostgreSQL',
	'Python-2.0',
	'Unlicense',
	'UPL-1.0',
	'Zlib',
])

type EnvironmentFilterValue =
	| 'client'
	| 'server'
	| 'client_and_server'
	| 'singleplayer'

function getEnvironmentFilterValue(
	environment?: string,
): EnvironmentFilterValue | undefined {
	switch (environment) {
		case 'client_only':
			return 'client'
		case 'server_only':
		case 'dedicated_server_only':
			return 'server'
		case 'client_and_server':
		case 'client_only_server_optional':
		case 'server_only_client_optional':
		case 'client_or_server':
		case 'client_or_server_prefers_both':
			return 'client_and_server'
		case 'singleplayer_only':
			return 'singleplayer'
		default:
			return undefined
	}
}

function isOpenSource(item: ContentItem): boolean {
	const licenseId = item.project?.license?.id.replace(/-(?:only|or-later)$/, '')
	return !!licenseId && openSourceLicenseIds.has(licenseId)
}

const loaderKeys = new Set(['fabric', 'forge', 'neoforge', 'quilt'])

const messages = defineMessages({
	categoryState: {
		id: 'content.metadata-filter.state',
		defaultMessage: '状态',
	},
	categoryUpdates: {
		id: 'content.metadata-filter.updates',
		defaultMessage: '更新',
	},
	categoryType: {
		id: 'content.metadata-filter.type',
		defaultMessage: '类型',
	},
	categoryAuthor: {
		id: 'content.metadata-filter.author',
		defaultMessage: '作者',
	},
	categoryEnvironment: {
		id: 'content.metadata-filter.environment',
		defaultMessage: '环境',
	},
	categoryLoader: {
		id: 'content.metadata-filter.loader',
		defaultMessage: '加载器',
	},
	categorySource: {
		id: 'content.metadata-filter.source',
		defaultMessage: '来源',
	},
	categoryExternal: {
		id: 'content.metadata-filter.external',
		defaultMessage: '外部文件',
	},
	categoryOpenSource: {
		id: 'content.metadata-filter.open-source',
		defaultMessage: '开源',
	},
	optionEnabled: {
		id: 'content.metadata-filter.state.enabled',
		defaultMessage: '已启用',
	},
	optionDisabled: {
		id: 'content.metadata-filter.state.disabled',
		defaultMessage: '已禁用',
	},
	optionUpdateAvailable: {
		id: 'content.metadata-filter.update.available',
		defaultMessage: '可更新',
	},
	optionUpToDate: {
		id: 'content.metadata-filter.update.up-to-date',
		defaultMessage: '已是最新',
	},
	optionUnknown: {
		id: 'content.metadata-filter.unknown',
		defaultMessage: '未知',
	},
	optionClient: {
		id: 'content.metadata-filter.environment.client',
		defaultMessage: '客户端',
	},
	optionServer: {
		id: 'content.metadata-filter.environment.server',
		defaultMessage: '服务端',
	},
	optionClientAndServer: {
		id: 'content.metadata-filter.environment.client-and-server',
		defaultMessage: '客户端和服务端',
	},
	optionSingleplayer: {
		id: 'content.metadata-filter.environment.singleplayer',
		defaultMessage: '单人游戏',
	},
	optionOtherLoader: {
		id: 'content.metadata-filter.loader.other',
		defaultMessage: '其他',
	},
	optionSourceLocal: {
		id: 'content.metadata-filter.source.local',
		defaultMessage: '本地',
	},
	optionSourceCurseforge: {
		id: 'content.metadata-filter.source.curseforge',
		defaultMessage: 'CurseForge',
	},
	optionSourceModrinthModpack: {
		id: 'content.metadata-filter.source.modrinth-modpack',
		defaultMessage: 'Modrinth 整合包',
	},
	optionSourceImportedModpack: {
		id: 'content.metadata-filter.source.imported-modpack',
		defaultMessage: '导入整合包',
	},
	optionSourceServerProject: {
		id: 'content.metadata-filter.source.server-project',
		defaultMessage: '服务器项目',
	},
	optionSourceSharedInstance: {
		id: 'content.metadata-filter.source.shared-instance',
		defaultMessage: '共享实例',
	},
	optionExternal: {
		id: 'content.metadata-filter.external.external',
		defaultMessage: '外部文件',
	},
	optionLinked: {
		id: 'content.metadata-filter.external.linked',
		defaultMessage: '在线项目',
	},
	optionOpenSource: {
		id: 'content.metadata-filter.open-source.open',
		defaultMessage: '开源',
	},
	optionClosedSource: {
		id: 'content.metadata-filter.open-source.closed',
		defaultMessage: '非开源',
	},
	searchPlaceholder: {
		id: 'content.metadata-filter.search',
		defaultMessage: '搜索...',
	},
	clearLabel: {
		id: 'content.metadata-filter.clear',
		defaultMessage: '清除',
	},
})

// ---- window 级内存持久化（导航切换保留，关软件丢弃） ----

const memory: Record<string, Map<string, unknown>> = ((
	window as unknown as { __ctMemory?: Record<string, Map<string, unknown>> }
).__ctMemory ??= {})
function getMap<K, V>(namespace: string): Map<K, V> {
	if (!memory[namespace]) memory[namespace] = new Map<string, unknown>()
	return memory[namespace] as Map<K, V>
}

export function useContentMetadataFilters(
	items: Ref<ContentItem[]> | ComputedRef<ContentItem[]>,
	persistKey?: string,
) {
	const { formatMessage } = useVIntl()

	const definitions = computed<MetadataFilterDefinition[]>(() => {
		const typeMessages = commonProjectTypeCategoryMessages
		return [
			{
				key: 'state',
				label: formatMessage(messages.categoryState),
				order: ['enabled', 'disabled'],
				values: (item) =>
					item.enabled === undefined
						? []
						: [item.enabled ? 'enabled' : 'disabled'],
				labelFor: (value) =>
					value === 'enabled'
						? formatMessage(messages.optionEnabled)
						: formatMessage(messages.optionDisabled),
			},
			{
				key: 'updates',
				label: formatMessage(messages.categoryUpdates),
				order: ['available', 'current'],
				values: (item) => [
					item.update != null ? 'available' : 'current',
				],
				labelFor: (value) =>
					value === 'available'
						? formatMessage(messages.optionUpdateAvailable)
						: formatMessage(messages.optionUpToDate),
			},
			{
				key: 'type',
				label: formatMessage(messages.categoryType),
				values: (item) => [normalizeProjectType(item.project_type)],
				labelFor: (value) => {
					const message =
						typeMessages[value as keyof typeof typeMessages]
					return message ? formatMessage(message) : value
				},
			},
			{
				key: 'author',
				label: formatMessage(messages.categoryAuthor),
				searchable: true,
				values: (item) =>
					item.owner?.name ? [item.owner.name] : [UNKNOWN],
				labelFor: (value) =>
					value === UNKNOWN
						? formatMessage(messages.optionUnknown)
						: value,
			},
			{
				key: 'environment',
				label: formatMessage(messages.categoryEnvironment),
				order: ['client', 'server', 'client_and_server', 'singleplayer'],
				values: (item) => {
					const value = getEnvironmentFilterValue(item.environment)
					return value ? [value] : [UNKNOWN]
				},
				labelFor: (value) => {
					switch (value) {
						case 'client':
							return formatMessage(messages.optionClient)
						case 'server':
							return formatMessage(messages.optionServer)
						case 'client_and_server':
							return formatMessage(messages.optionClientAndServer)
						case 'singleplayer':
							return formatMessage(messages.optionSingleplayer)
						default:
							return formatMessage(messages.optionUnknown)
					}
				},
			},
			{
				key: 'loader',
				label: formatMessage(messages.categoryLoader),
				order: ['fabric', 'forge', 'neoforge', 'quilt'],
				values: (item) => {
					if (!item.loader) return [UNKNOWN]
					return loaderKeys.has(item.loader) ? [item.loader] : ['other']
				},
				labelFor: (value) => {
					switch (value) {
						case 'fabric':
							return 'Fabric'
						case 'forge':
							return 'Forge'
						case 'neoforge':
							return 'NeoForge'
						case 'quilt':
							return 'Quilt'
						case 'other':
							return formatMessage(messages.optionOtherLoader)
						default:
							return formatMessage(messages.optionUnknown)
					}
				},
			},
			{
				key: 'source',
				label: formatMessage(messages.categorySource),
				order: [
					'local',
					'curseforge',
					'modrinth_modpack',
					'imported_modpack',
					'server_project',
					'shared_instance',
				],
				values: (item) =>
					item.source_kind ? [item.source_kind] : [UNKNOWN],
				labelFor: (value) => {
					switch (value) {
						case 'local':
							return formatMessage(messages.optionSourceLocal)
						case 'curseforge':
							return formatMessage(messages.optionSourceCurseforge)
						case 'modrinth_modpack':
							return formatMessage(
								messages.optionSourceModrinthModpack,
							)
						case 'imported_modpack':
							return formatMessage(
								messages.optionSourceImportedModpack,
							)
						case 'server_project':
							return formatMessage(
								messages.optionSourceServerProject,
							)
						case 'shared_instance':
							return formatMessage(
								messages.optionSourceSharedInstance,
							)
						default:
							return formatMessage(messages.optionUnknown)
					}
				},
			},
			{
				key: 'external',
				label: formatMessage(messages.categoryExternal),
				order: ['linked', 'external'],
				values: (item) => [
					item.external ? 'external' : 'linked',
				],
				labelFor: (value) =>
					value === 'external'
						? formatMessage(messages.optionExternal)
						: formatMessage(messages.optionLinked),
			},
			{
				key: 'open_source',
				label: formatMessage(messages.categoryOpenSource),
				order: ['open', 'closed'],
				values: (item) => {
					if (isOpenSource(item)) return ['open']
					return item.project?.license ? ['closed'] : [UNKNOWN]
				},
				labelFor: (value) => {
					switch (value) {
						case 'open':
							return formatMessage(messages.optionOpenSource)
						case 'closed':
							return formatMessage(messages.optionClosedSource)
						default:
							return formatMessage(messages.optionUnknown)
					}
				},
			},
		]
	})

	const metadataFilterCategories = computed<MetadataFilterCategory[]>(() =>
		definitions.value
			.map((definition) => {
				const options = new Map<string, MetadataFilterOption>()
				const counts = new Map<string, number>()
				for (const item of items.value) {
					const seen = new Set<string>()
					for (const value of definition.values(item)) {
						if (seen.has(value)) continue
						seen.add(value)
						if (!options.has(value)) {
							options.set(value, {
								value,
								label: definition.labelFor(value),
							})
						}
						counts.set(value, (counts.get(value) ?? 0) + 1)
					}
				}

				const total = items.value.length
				const visible = [...options.values()]
					.filter(
						(option) => (counts.get(option.value) ?? 0) !== total,
					)
					.sort((a, b) => {
						if (a.value === UNKNOWN) return 1
						if (b.value === UNKNOWN) return -1
						const order = definition.order
						if (order) {
							const indexA = order.indexOf(a.value)
							const indexB = order.indexOf(b.value)
							if (indexA !== -1 && indexB !== -1) {
								return indexA - indexB
							}
							if (indexA !== -1) return -1
							if (indexB !== -1) return 1
						}
						return a.label.localeCompare(b.label, undefined, {
							numeric: true,
						})
					})

				return {
					key: definition.key,
					label: definition.label,
					searchable: definition.searchable,
					options: visible,
				}
			})
			.filter((category) => category.options.length > 0),
	)

	// ---- 选择状态（排除式：勾选 = 显示，取消勾选 = 隐藏；默认全部勾选） ----

	const memory = getMap<string, Record<string, string[]>>('metadataFilters')
	const excluded = ref<Record<string, string[]>>(
		persistKey ? (memory.get(persistKey) ?? {}) : {},
	)

	function optionsByKey(key: string): MetadataFilterOption[] {
		return (
			metadataFilterCategories.value.find(
				(category) => category.key === key,
			)?.options ?? []
		)
	}

	function getExcludedSet(key: string): Set<string> {
		return new Set(excluded.value[key] ?? [])
	}

	function getSelectedValues(key: string): string[] {
		const excludedSet = getExcludedSet(key)
		return optionsByKey(key)
			.filter((option) => !excludedSet.has(option.value))
			.map((option) => option.value)
	}

	function setCategorySelection(key: string, selectedValues: string[]) {
		const selectedSet = new Set(selectedValues)
		const nextExcluded = optionsByKey(key)
			.filter((option) => !selectedSet.has(option.value))
			.map((option) => option.value)
		if (nextExcluded.length === 0) {
			const { [key]: _removed, ...rest } = excluded.value
			excluded.value = rest
		} else {
			excluded.value = { ...excluded.value, [key]: nextExcluded }
		}
	}

	function getExcludedCount(key: string): number {
		return excluded.value[key]?.length ?? 0
	}

	/** A category filters only while at least one option stays selected; an
	 * empty selection means "no filter" (everything is shown). */
	function isCategoryFiltering(key: string): boolean {
		const excludedSet = getExcludedSet(key)
		return optionsByKey(key).some((option) => !excludedSet.has(option.value))
	}

	// 选项变化时修剪失效的排除值（选项消失 → 自动从排除集移除）。
	watch(
		metadataFilterCategories,
		(categories) => {
			if (categories.length === 0) return
			let changed = false
			const next: Record<string, string[]> = {}
			for (const [key, values] of Object.entries(excluded.value)) {
				const valid = new Set(optionsByKey(key).map((o) => o.value))
				const pruned = values.filter((value) => valid.has(value))
				if (pruned.length > 0) next[key] = pruned
				if (pruned.length !== values.length) changed = true
			}
			if (changed) excluded.value = next
		},
		{ immediate: true },
	)

	watch(
		excluded,
		(value) => {
			if (persistKey) memory.set(persistKey, value)
		},
		{ deep: true },
	)

	function applyMetadataFilters(source: ContentItem[]): ContentItem[] {
		const active = definitions.value.filter((definition) =>
			isCategoryFiltering(definition.key),
		)
		if (active.length === 0) return source

		return source.filter((item) =>
			active.every((definition) => {
				const excludedSet = getExcludedSet(definition.key)
				return definition
					.values(item)
					.some((value) => !excludedSet.has(value))
			}),
		)
	}

	return {
		metadataFilterCategories,
		getSelectedValues,
		setCategorySelection,
		getExcludedCount,
		isCategoryFiltering,
		applyMetadataFilters,
	}
}
