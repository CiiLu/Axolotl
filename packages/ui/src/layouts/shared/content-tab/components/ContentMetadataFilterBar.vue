<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'

import MultiSelect from '#ui/components/base/MultiSelect.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

import type { MetadataFilterCategory } from '../composables'

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

const filterScrollRef = ref<HTMLElement | null>(null)
let filterScrollWheelHandler: ((event: WheelEvent) => void) | null = null

// 平滑滚动：滚轮累积目标位置，由 rAF 逐帧插值逼近，避免瞬时跳变
let scrollTarget: number | null = null
let scrollAnimationFrame: number | null = null

function animateFilterScroll() {
	const container = filterScrollRef.value
	if (!container || scrollTarget === null) return
	const maxScroll = Math.max(container.scrollWidth - container.clientWidth, 0)
	// 同步 clamp 目标，防止宽度变化导致目标越界
	scrollTarget = Math.min(Math.max(scrollTarget, 0), maxScroll)
	const clamped = scrollTarget
	const current = container.scrollLeft
	const diff = clamped - current
	if (Math.abs(diff) < 0.5) {
		container.scrollLeft = clamped
		scrollTarget = null
		scrollAnimationFrame = null
		return
	}
	container.scrollLeft = current + diff * 0.25
	scrollAnimationFrame = requestAnimationFrame(animateFilterScroll)
}

function cancelFilterScrollAnimation() {
	if (scrollAnimationFrame !== null) {
		cancelAnimationFrame(scrollAnimationFrame)
		scrollAnimationFrame = null
	}
	scrollTarget = null
}

const suppressFilterHoverOpen = ref(false)
let filterHoverSuppressTimer: ReturnType<typeof setTimeout> | null = null

// 自绘悬浮滚动条：不占布局空间，悬停容器时显示
const scrollbarThumbRef = ref<HTMLElement | null>(null)
let scrollbarObserver: ResizeObserver | null = null

function updateFilterScrollbar() {
	const container = filterScrollRef.value
	const thumb = scrollbarThumbRef.value
	if (!container || !thumb) return
	const maxScroll = container.scrollWidth - container.clientWidth
	if (maxScroll <= 0) {
		thumb.style.opacity = '0'
		return
	}
	const track = container.clientWidth
	const thumbWidth = Math.max(24, (track / container.scrollWidth) * track)
	thumb.style.width = `${thumbWidth}px`
	thumb.style.transform = `translateX(${(container.scrollLeft / maxScroll) * (track - thumbWidth)}px)`
	// 清除内联隐藏，交给 group-hover 控制显隐
	thumb.style.opacity = ''
}

function handleFilterStripScroll() {
	suppressFilterHoverOpen.value = true
	if (filterHoverSuppressTimer) clearTimeout(filterHoverSuppressTimer)
	filterHoverSuppressTimer = setTimeout(() => {
		suppressFilterHoverOpen.value = false
		filterHoverSuppressTimer = null
	}, 500)
	updateFilterScrollbar()
}

watch(
	filterScrollRef,
	(container, previous) => {
		if (previous && filterScrollWheelHandler) {
			previous.removeEventListener('wheel', filterScrollWheelHandler)
			filterScrollWheelHandler = null
		}
		if (scrollbarObserver) {
			scrollbarObserver.disconnect()
			scrollbarObserver = null
		}
		if (!container) return

		// 滚轮始终转换为容器横向滚动：只要容器可滚动就一直拦截，
		// 到边界也只是停在原地，绝不放行给页面滚动。目标累积 + rAF 平滑插值；
		// 累积后立即 clamp，避免滚过头时目标残留超额位移导致反向滚动迟滞。
		filterScrollWheelHandler = (event) => {
			if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return
			if (container.scrollWidth <= container.clientWidth + 1) return
			event.preventDefault()
			const delta = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY
			const maxScroll = Math.max(container.scrollWidth - container.clientWidth, 0)
			scrollTarget = Math.min(
				Math.max((scrollTarget ?? container.scrollLeft) + delta * 0.5, 0),
				maxScroll,
			)
			if (scrollAnimationFrame === null) {
				scrollAnimationFrame = requestAnimationFrame(animateFilterScroll)
			}
		}
		container.addEventListener('wheel', filterScrollWheelHandler, {
			passive: false,
		})

		scrollbarObserver = new ResizeObserver(updateFilterScrollbar)
		scrollbarObserver.observe(container)
		updateFilterScrollbar()
	},
	{ immediate: true },
)

onBeforeUnmount(() => {
	if (filterScrollRef.value && filterScrollWheelHandler) {
		filterScrollRef.value.removeEventListener('wheel', filterScrollWheelHandler)
		filterScrollWheelHandler = null
	}
	cancelFilterScrollAnimation()
	if (scrollbarObserver) {
		scrollbarObserver.disconnect()
		scrollbarObserver = null
	}
	if (filterHoverSuppressTimer) {
		clearTimeout(filterHoverSuppressTimer)
		filterHoverSuppressTimer = null
	}
})
</script>

<template>
	<div class="group relative flex min-w-0 flex-1 items-center">
		<div
			ref="filterScrollRef"
			class="content-filter-scroll flex w-full min-w-0 flex-nowrap items-center gap-1.5 px-1.5"
			@scroll="handleFilterStripScroll"
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
				:active="(props.modelValue[category.key] ?? []).length === category.options.length"
				:dropdown-min-width="'15rem'"
				:checkbox-position="'left'"
				:hover-open="!suppressFilterHoverOpen"
				show-selection-actions
				:selection-actions-clear-label="formatMessage(messages.clear)"
				:selection-actions-select-all-label="formatMessage(messages.selectAll)"
				@update:model-value="(values) => emit('update:category', category.key, values)"
			>
				<template #input-content>
					<span class="flex items-center gap-1.5 text-sm font-semibold">
						<span class="truncate">{{ category.label }}</span>
						<span
							v-if="props.filteringKeys.includes(category.key)"
							class="rounded-full bg-brand-highlight px-1.5 text-xs font-normal tabular-nums text-brand"
						>
							{{ (props.modelValue[category.key] ?? []).length }}/{{ category.options.length }}
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
