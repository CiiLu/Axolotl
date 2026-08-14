<script setup lang="ts">
import { FilterIcon } from '@modrinth/assets'
import { ref, watch } from 'vue'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'
import MultiSelect from '#ui/components/base/MultiSelect.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

import { type MetadataFilterCategory, useHorizontalFilterScroll } from '../composables'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	searchPlaceholder: {
		id: 'content.metadata-filter.search',
		defaultMessage: 'Search...',
	},
	clear: {
		id: 'content.metadata-filter.clear',
		defaultMessage: 'Clear',
	},
	selectAll: {
		id: 'content.metadata-filter.select-all',
		defaultMessage: 'Select all',
	},
	filterToggle: {
		id: 'content.metadata-filter.toggle',
		defaultMessage: 'Filter',
	},
})

const props = withDefaults(
	defineProps<{
		categories: MetadataFilterCategory[]
		modelValue: Record<string, string[]>
		filteringKeys?: string[]
	}>(),
	{
		filteringKeys: () => [],
	},
)

const emit = defineEmits<{
	'update:category': [key: string, values: string[]]
}>()

/** 筛选条折叠状态：默认折叠，点击筛选按钮展开/收起。 */
const expanded = ref(false)

// 收起筛选器时自动重置筛选（全选）
watch(expanded, (isExpanded) => {
	if (isExpanded) return
	for (const category of props.categories) {
		emit(
			'update:category',
			category.key,
			category.options.map((option) => option.value),
		)
	}
})

const filterScrollRef = ref<HTMLElement | null>(null)
const scrollbarThumbRef = ref<HTMLElement | null>(null)
const { suppressHoverOpen, handleScroll } = useHorizontalFilterScroll(
	filterScrollRef,
	scrollbarThumbRef,
)

function selectedCount(category: MetadataFilterCategory): number {
	return (props.modelValue[category.key] ?? []).length
}

function isCategoryFiltering(category: MetadataFilterCategory): boolean {
	return props.filteringKeys.includes(category.key)
}
</script>

<template>
	<div class="group relative flex min-w-0 flex-1 items-center gap-1.5">
		<ButtonStyled
			circular
			:type="expanded ? 'chip' : 'transparent'"
			:color="expanded ? 'brand' : 'standard'"
			color-fill="text"
			hover-color-fill="background"
		>
			<button
				v-tooltip="formatMessage(messages.filterToggle)"
				:aria-label="formatMessage(messages.filterToggle)"
				:aria-expanded="expanded"
				@click="expanded = !expanded"
			>
				<FilterIcon />
			</button>
		</ButtonStyled>

		<div
			class="grid min-w-0 flex-1 transition-[grid-template-columns] duration-300 ease-in-out"
			:class="expanded ? 'grid-cols-[1fr]' : 'grid-cols-[0fr]'"
		>
			<div class="relative min-w-0 overflow-hidden">
				<div
					ref="filterScrollRef"
					class="content-filter-scroll flex w-full min-w-0 flex-nowrap items-center gap-1.5 px-1.5"
					@scroll="handleScroll"
				>
					<MultiSelect
						v-for="category in props.categories"
						:key="category.key"
						:model-value="props.modelValue[category.key] ?? []"
						:options="category.options"
						:max-height="420"
						:clearable="false"
						:show-chevron="false"
						:fit-content="true"
						:searchable="category.searchable"
						:search-placeholder="formatMessage(messages.searchPlaceholder)"
						:trigger-class="'h-8 shrink-0 !rounded-full border-0 px-2.5 transition-all hover:brightness-110 active:brightness-110'"
						:active="selectedCount(category) === category.options.length"
						:dropdown-min-width="'15rem'"
						:checkbox-position="'left'"
						:hover-open="!suppressHoverOpen"
						show-selection-actions
						:selection-actions-clear-label="formatMessage(messages.clear)"
						:selection-actions-select-all-label="formatMessage(messages.selectAll)"
						@update:model-value="(values) => emit('update:category', category.key, values)"
					>
						<template #input-content>
							<span class="flex items-center gap-1.5 text-sm font-semibold">
								<span class="truncate">{{ category.label }}</span>
								<span
									v-if="isCategoryFiltering(category)"
									class="rounded-full bg-brand-highlight px-1.5 text-xs font-normal tabular-nums text-brand"
								>
									{{ selectedCount(category) }}/{{ category.options.length }}
								</span>
							</span>
						</template>
					</MultiSelect>
				</div>

				<div
					ref="scrollbarThumbRef"
					class="pointer-events-none absolute bottom-0 left-0 z-10 h-[3px] rounded-full bg-surface-5 opacity-0 transition-opacity duration-150 group-hover:opacity-100"
				/>
			</div>
		</div>
	</div>
</template>

<style scoped>
/* 隐藏原生滚动条（不占布局空间），滚动条由自绘悬浮条替代 */
.content-filter-scroll {
	overflow-x: auto;
	scrollbar-width: none;
	-ms-overflow-style: none;
}

.content-filter-scroll::-webkit-scrollbar {
	display: none;
}
</style>
