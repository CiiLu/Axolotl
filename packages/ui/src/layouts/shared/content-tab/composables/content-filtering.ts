import { useSessionStorage } from '@vueuse/core'
import type { Ref } from 'vue'
import { computed, ref, watch } from 'vue'

import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonProjectTypeCategoryMessages, normalizeProjectType } from '#ui/utils/common-messages'

import type { ClientWarningType, ContentItem } from '../types'

const CLIENT_ONLY_ENVIRONMENTS = new Set(['client_only', 'singleplayer_only'])

export function isClientOnlyEnvironment(env?: string | null): boolean {
	return !!env && CLIENT_ONLY_ENVIRONMENTS.has(env)
}

export function getClientWarningType(item: ContentItem): ClientWarningType | null {
	if (item.pack_client_retained) return 'retained'
	if (item.pack_client_depends) return 'depends'
	if (isClientOnlyEnvironment(item.environment)) return 'environment'
	return null
}

export interface ContentFilterOption {
	id: string
	label: string
}

export interface ContentFilterConfig {
	showTypeFilters?: boolean
	showUpdateFilter?: boolean
	showWarningsFilter?: boolean
	isPackLocked?: Ref<boolean>
	persistKey?: string
}

const messages = defineMessages({
	updates: {
		id: 'content.filter.updates',
		defaultMessage: '可更新',
	},
	warnings: {
		id: 'content.filter.warnings',
		defaultMessage: 'Warnings',
	},
	enabled: {
		id: 'content.filter.enabled',
		defaultMessage: 'Enabled',
	},
	disabled: {
		id: 'content.filter.disabled',
		defaultMessage: 'Disabled',
	},
})

export function useContentFilters(items: Ref<ContentItem[]>, config?: ContentFilterConfig) {
	const { formatMessage } = useVIntl()

	const selectedTypeFilter = config?.persistKey
		? useSessionStorage<string | null>(`content-filters-type:${config.persistKey}`, null)
		: ref<string | null>(null)

	const selectedStatusFilters = config?.persistKey
		? useSessionStorage<string[]>(`content-filters-status:${config.persistKey}`, [])
		: ref<string[]>([])

	const typeFilteredItems = computed(() => {
		if (!selectedTypeFilter.value) return items.value
		return items.value.filter(
			(item) => normalizeProjectType(item.project_type) === selectedTypeFilter.value,
		)
	})

	const availableStatusFilters = computed<Array<'enabled' | 'disabled'>>(() => {
		const source = typeFilteredItems.value
		const hasEnabledContent = source.some((m) => m.enabled)
		const hasDisabledContent = source.some((m) => !m.enabled)

		return hasEnabledContent && hasDisabledContent ? ['enabled', 'disabled'] : []
	})

	const row1FilterOptions = computed<ContentFilterOption[]>(() => {
		const options: ContentFilterOption[] = []

		if (config?.showTypeFilters) {
			const frequency = items.value.reduce((map: Record<string, number>, item) => {
				const normalized = normalizeProjectType(item.project_type)
				map[normalized] = (map[normalized] || 0) + 1
				return map
			}, {})
			const types = Object.keys(frequency).sort((a, b) => frequency[b] - frequency[a])
			for (const type of types) {
				const msg =
					commonProjectTypeCategoryMessages[type as keyof typeof commonProjectTypeCategoryMessages]
				const label = msg ? formatMessage(msg) : type.charAt(0).toUpperCase() + type.slice(1) + 's'
				options.push({ id: type, label })
			}
		}

		return options
	})

	const row2FilterOptions = computed<ContentFilterOption[]>(() => {
		const source = typeFilteredItems.value
		const options: ContentFilterOption[] = []

		if (config?.showUpdateFilter && source.some((m) => m.has_update)) {
			options.push({ id: 'updates', label: formatMessage(messages.updates) })
		}

		if (config?.showWarningsFilter && source.some((m) => getClientWarningType(m) !== null)) {
			options.push({ id: 'warnings', label: formatMessage(messages.warnings) })
		}

		for (const status of availableStatusFilters.value) {
			options.push({
				id: status,
				label: formatMessage(status === 'enabled' ? messages.enabled : messages.disabled),
			})
		}

		return options
	})

	const allFilterOptions = computed<ContentFilterOption[]>(() => {
		return [...row1FilterOptions.value, ...row2FilterOptions.value]
	})

	const totalCount = computed(() => items.value.length)

	const filterCounts = computed(() => {
		const counts: Record<string, number> = {}

		for (const item of items.value) {
			const type = normalizeProjectType(item.project_type)
			counts[type] = (counts[type] || 0) + 1
		}

		const source = typeFilteredItems.value

		counts['updates'] = source.filter((m) => m.has_update).length
		counts['enabled'] = source.filter((m) => m.enabled).length
		counts['disabled'] = source.filter((m) => !m.enabled).length
		counts['warnings'] = source.filter((m) => getClientWarningType(m) !== null).length

		return counts
	})

	watch(
		allFilterOptions,
		() => {
			const validIds = new Set(allFilterOptions.value.map((opt) => opt.id))
			if (selectedTypeFilter.value && !validIds.has(selectedTypeFilter.value)) {
				selectedTypeFilter.value = null
			}
			selectedStatusFilters.value = selectedStatusFilters.value.filter((f) => validIds.has(f))
		},
		{ immediate: true },
	)

	function toggleTypeFilter(filterId: string) {
		if (selectedTypeFilter.value !== filterId) {
			selectedTypeFilter.value = filterId
		}
	}

	function toggleStatusFilter(filterId: string) {
		if (filterId === 'enabled' || filterId === 'disabled') {
			const index = selectedStatusFilters.value.indexOf(filterId)
			const otherStatusFilter = filterId === 'enabled' ? 'disabled' : 'enabled'
			if (index === -1) {
				selectedStatusFilters.value = [
					...selectedStatusFilters.value.filter((filter) => filter !== otherStatusFilter),
					filterId,
				]
			} else {
				selectedStatusFilters.value.splice(index, 1)
			}
			return
		}

		const index = selectedStatusFilters.value.indexOf(filterId)
		if (index === -1) {
			selectedStatusFilters.value.push(filterId)
		} else {
			selectedStatusFilters.value.splice(index, 1)
		}
	}

	function applyFilters(source: ContentItem[]): ContentItem[] {
		let result = source

		if (selectedTypeFilter.value) {
			result = result.filter(
				(item) => normalizeProjectType(item.project_type) === selectedTypeFilter.value,
			)
		}

		if (selectedStatusFilters.value.length > 0) {
			result = result.filter((item) => {
				for (const filter of selectedStatusFilters.value) {
					if (filter === 'updates' && !item.has_update) return false
					if (filter === 'enabled' && !item.enabled) return false
					if (filter === 'disabled' && item.enabled) return false
					if (filter === 'warnings' && getClientWarningType(item) === null) return false
				}
				return true
			})
		}

		return result
	}

	return {
		selectedTypeFilter,
		selectedStatusFilters,
		row1FilterOptions,
		row2FilterOptions,
		totalCount,
		filterCounts,
		toggleTypeFilter,
		toggleStatusFilter,
		applyFilters,
	}
}
