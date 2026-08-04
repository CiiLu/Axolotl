<script setup lang="ts">
import {
	CheckIcon,
	ChevronDownIcon,
	ChevronUpIcon,
	ExpandIcon,
	GripVerticalIcon,
	MoreVerticalIcon,
	PencilIcon,
	PlusIcon,
	RefreshCwIcon,
	RotateCounterClockwiseIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	OverflowMenu,
	useVIntl,
} from '@modrinth/ui'
import { useElementSize } from '@vueuse/core'
import { computed, ref } from 'vue'
import Draggable from 'vuedraggable'

import {
	addHomeWidget,
	getHomeGridColumnCount,
	HOME_WIDGET_SIZE_OPTIONS,
	moveHomeWidget,
	packHomeWidgets,
	removeHomeWidget,
	replaceHomeDashboardWidgets,
	resizeHomeWidget,
	type HomeDashboardConfig,
	type HomeWidgetPlacement,
	type HomeWidgetSize,
} from '@/components/home/home-dashboard'
import { provideHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import HomeCalendar from '@/components/home/HomeCalendar.vue'
import HomeGreeting from '@/components/home/HomeGreeting.vue'
import HomePinnedInstances from '@/components/home/HomePinnedInstances.vue'
import HomePinnedServers from '@/components/home/HomePinnedServers.vue'
import HomePinnedWorlds from '@/components/home/HomePinnedWorlds.vue'
import HomeRecentWorlds from '@/components/home/HomeRecentWorlds.vue'
import HomeShortcutWidget from '@/components/home/HomeShortcutWidget.vue'
import HomeWidgetPickerModal from '@/components/home/HomeWidgetPickerModal.vue'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	config: HomeDashboardConfig
	instances: GameInstance[]
	playerName: string | null
}>()

const emit = defineEmits<{
	change: [config: HomeDashboardConfig]
	reset: []
}>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
provideHomeDashboardRuntime(handleError)
const editing = ref(false)
const gridContainer = ref<HTMLElement>()
const widgetPicker = ref<InstanceType<typeof HomeWidgetPickerModal>>()
const replacingWidgetId = ref<string | null>(null)
const { width } = useElementSize(gridContainer)
const columnCount = computed(() => getHomeGridColumnCount(width.value))
const packedWidgets = computed(() => packHomeWidgets(props.config.widgets, columnCount.value))
const packedById = computed(() => new Map(packedWidgets.value.map((widget) => [widget.id, widget])))

const messages = defineMessages({
	customize: { id: 'app.home.widgets.customize', defaultMessage: 'Customize widgets' },
	done: { id: 'app.home.widgets.done', defaultMessage: 'Finish editing' },
	add: { id: 'app.home.widgets.add', defaultMessage: 'Add widget' },
	reset: { id: 'app.home.widgets.reset', defaultMessage: 'Restore default widgets' },
	options: { id: 'app.home.widgets.options', defaultMessage: 'Widget options' },
	moveEarlier: { id: 'app.home.widgets.move-earlier', defaultMessage: 'Move earlier' },
	moveLater: { id: 'app.home.widgets.move-later', defaultMessage: 'Move later' },
	remove: { id: 'app.home.widgets.remove', defaultMessage: 'Remove widget' },
	drag: { id: 'app.home.widgets.drag', defaultMessage: 'Drag to move widget' },
	replace: { id: 'app.home.widgets.replace', defaultMessage: 'Replace target' },
	empty: { id: 'app.home.widgets.empty', defaultMessage: 'Add a widget to build your Home.' },
	size: { id: 'app.home.widgets.size', defaultMessage: 'Size {size}' },
})

const widgetsModel = computed({
	get: () => props.config.widgets,
	set: (widgets: HomeWidgetPlacement[]) =>
		emit('change', replaceHomeDashboardWidgets(props.config, widgets)),
})

function widgetStyle(widget: HomeWidgetPlacement) {
	const packed = packedById.value.get(widget.id)
	if (!packed) return undefined
	return {
		gridColumn: `${packed.column} / span ${packed.effectiveColumns}`,
		gridRow: `${packed.row} / span ${packed.effectiveRows}`,
	}
}

function effectiveSize(widget: HomeWidgetPlacement): HomeWidgetSize {
	const packed = packedById.value.get(widget.id)
	return packed
		? (`${packed.effectiveColumns}x${packed.effectiveRows}` as HomeWidgetSize)
		: widget.size
}

function openWidgetPicker() {
	replacingWidgetId.value = null
	widgetPicker.value?.show()
}

function addWidget(widget: HomeWidgetPlacement) {
	const replacingId = replacingWidgetId.value
	replacingWidgetId.value = null
	if (!replacingId) {
		emit('change', addHomeWidget(props.config, widget))
		return
	}

	emit(
		'change',
		replaceHomeDashboardWidgets(
			props.config,
			props.config.widgets.map((current) =>
				current.id === replacingId ? { ...widget, id: current.id, size: current.size } : current,
			),
		),
	)
}

function replaceWidgetTarget(widget: HomeWidgetPlacement) {
	replacingWidgetId.value = widget.id
	widgetPicker.value?.show(widget.kind)
}

function removeWidget(id: string) {
	emit('change', removeHomeWidget(props.config, id))
}

function resizeWidget(id: string, size: HomeWidgetSize) {
	emit('change', resizeHomeWidget(props.config, id, size))
}

function moveWidget(index: number, direction: -1 | 1) {
	emit('change', moveHomeWidget(props.config, index, direction))
}

function widgetOptions(widget: HomeWidgetPlacement, index: number) {
	return [
		...HOME_WIDGET_SIZE_OPTIONS[widget.kind].map((size) => ({
			id: `size-${size}`,
			icon: ExpandIcon,
			disabled: widget.size === size,
			action: () => resizeWidget(widget.id, size),
		})),
		...(widget.target
			? [
					{ divider: true },
					{
						id: 'replace',
						icon: RefreshCwIcon,
						action: () => replaceWidgetTarget(widget),
					},
				]
			: []),
		{ divider: true },
		{
			id: 'move-earlier',
			icon: ChevronUpIcon,
			disabled: index === 0,
			action: () => moveWidget(index, -1),
		},
		{
			id: 'move-later',
			icon: ChevronDownIcon,
			disabled: index === props.config.widgets.length - 1,
			action: () => moveWidget(index, 1),
		},
		{ divider: true },
		{ id: 'remove', icon: TrashIcon, color: 'red' as const, action: () => removeWidget(widget.id) },
	]
}
</script>

<template>
	<HomeWidgetPickerModal ref="widgetPicker" :instances="instances" @add="addWidget" />
	<section class="home-dashboard p-6 pb-20">
		<div class="home-dashboard-toolbar">
			<template v-if="editing">
				<ButtonStyled circular type="transparent">
					<button v-tooltip="formatMessage(messages.add)" @click="openWidgetPicker">
						<PlusIcon />
					</button>
				</ButtonStyled>
				<ButtonStyled circular type="transparent">
					<button v-tooltip="formatMessage(messages.reset)" @click="emit('reset')">
						<RotateCounterClockwiseIcon />
					</button>
				</ButtonStyled>
				<ButtonStyled circular>
					<button v-tooltip="formatMessage(messages.done)" @click="editing = false">
						<CheckIcon />
					</button>
				</ButtonStyled>
			</template>
			<ButtonStyled v-else circular type="transparent">
				<button
					v-tooltip="formatMessage(messages.customize)"
					data-onboarding-id="home-widget-customize"
					@click="editing = true"
				>
					<PencilIcon />
				</button>
			</ButtonStyled>
		</div>

		<div ref="gridContainer" class="mx-auto w-full max-w-[96rem]">
			<Draggable
				v-model="widgetsModel"
				item-key="id"
				tag="div"
				class="home-dashboard-grid"
				:class="{ 'is-editing': editing }"
				:style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))` }"
				handle=".home-widget-drag-handle"
				:animation="160"
				ghost-class="home-widget-ghost"
				data-onboarding-id="home-widget-grid"
			>
				<template #item="{ element: widget, index }">
					<article class="home-widget" :style="widgetStyle(widget)">
						<div v-if="editing" class="home-widget-edit-bar">
							<button
								v-tooltip="formatMessage(messages.drag)"
								type="button"
								class="home-widget-drag-handle"
							>
								<GripVerticalIcon />
							</button>
							<span class="text-xs font-semibold text-secondary">{{ widget.size }}</span>
							<ButtonStyled circular size="small" type="transparent" class="ml-auto">
								<OverflowMenu
									:options="widgetOptions(widget, index)"
									:tooltip="formatMessage(messages.options)"
								>
									<MoreVerticalIcon />
									<template
										v-for="size in HOME_WIDGET_SIZE_OPTIONS[widget.kind]"
										#[`size-${size}`]
										:key="size"
									>
										<ExpandIcon /> {{ formatMessage(messages.size, { size }) }}
									</template>
									<template #move-earlier>
										<ChevronUpIcon /> {{ formatMessage(messages.moveEarlier) }}
									</template>
									<template #move-later>
										<ChevronDownIcon /> {{ formatMessage(messages.moveLater) }}
									</template>
									<template #replace>
										<RefreshCwIcon /> {{ formatMessage(messages.replace) }}
									</template>
									<template #remove> <TrashIcon /> {{ formatMessage(messages.remove) }} </template>
								</OverflowMenu>
							</ButtonStyled>
						</div>
						<div class="home-widget-content">
							<HomeGreeting
								v-if="widget.kind === 'greeting'"
								:player-name="playerName"
								:dashboard-size="effectiveSize(widget)"
							/>
							<HomeRecentWorlds
								v-else-if="widget.kind === 'recent'"
								:instances="instances"
								dashboard
							/>
							<HomeCalendar
								v-else-if="widget.kind === 'calendar'"
								:instances="instances"
								:dashboard-size="effectiveSize(widget)"
							/>
							<HomePinnedInstances
								v-else-if="widget.kind === 'pinned-instances'"
								:instances="instances"
								dashboard
							/>
							<HomePinnedWorlds
								v-else-if="widget.kind === 'pinned-worlds'"
								:instances="instances"
								dashboard
							/>
							<HomePinnedServers
								v-else-if="widget.kind === 'pinned-servers'"
								:instances="instances"
								dashboard
							/>
							<HomeShortcutWidget v-else :placement="widget" :instances="instances" />
						</div>
					</article>
				</template>
			</Draggable>
			<div
				v-if="config.widgets.length === 0"
				class="flex min-h-64 flex-col items-center justify-center gap-4 rounded-lg border border-dashed border-divider text-center"
			>
				<p class="m-0 text-secondary">{{ formatMessage(messages.empty) }}</p>
				<ButtonStyled>
					<button @click="openWidgetPicker"><PlusIcon /> {{ formatMessage(messages.add) }}</button>
				</ButtonStyled>
			</div>
		</div>
	</section>
</template>

<style scoped>
.home-dashboard {
	container-type: inline-size;
}

.home-dashboard-toolbar {
	position: sticky;
	top: 0.75rem;
	z-index: 20;
	display: flex;
	width: fit-content;
	margin-left: auto;
	margin-bottom: 0.75rem;
	gap: 0.25rem;
	padding: 0.25rem;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	background: var(--surface-3);
	box-shadow: var(--shadow-button);
}

.home-dashboard-grid {
	display: grid;
	grid-auto-rows: 10rem;
	gap: 1rem;
}

.home-widget {
	display: flex;
	min-width: 0;
	min-height: 0;
	flex-direction: column;
	overflow: hidden;
	border: 1px solid var(--color-divider);
	border-radius: var(--radius-lg);
	background: var(--surface-3);
	box-shadow: var(--shadow-card);
}

.home-widget-edit-bar {
	display: flex;
	height: 2.25rem;
	flex: 0 0 auto;
	align-items: center;
	gap: 0.5rem;
	padding: 0 0.5rem;
	border-bottom: 1px solid var(--color-divider);
	background: var(--color-button-bg);
}

.home-widget-drag-handle {
	display: inline-flex;
	width: 2rem;
	height: 2rem;
	align-items: center;
	justify-content: center;
	padding: 0;
	border: 0;
	background: transparent;
	color: var(--color-secondary);
	cursor: grab;
}

.home-widget-drag-handle:active {
	cursor: grabbing;
}

.home-widget-content {
	min-width: 0;
	min-height: 0;
	flex: 1;
	overflow: hidden;
	padding: 1rem;
}

.home-widget-content > :deep(*) {
	height: 100%;
	min-height: 0;
}

.home-widget-ghost {
	opacity: 0.35;
}

@media (prefers-reduced-motion: reduce) {
	.home-dashboard-grid {
		scroll-behavior: auto;
	}
}
</style>
