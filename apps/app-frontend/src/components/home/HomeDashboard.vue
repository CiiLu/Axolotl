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
import { computed, ref, watch } from 'vue'
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
const dragging = ref(false)
const draggableWidgets = ref<HomeWidgetPlacement[]>([])
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

watch(
	() => props.config.widgets,
	(widgets) => {
		if (!dragging.value) draggableWidgets.value = [...widgets]
	},
	{ immediate: true, deep: true },
)

function widgetStyle(widget: HomeWidgetPlacement) {
	const packed = packedById.value.get(widget.id)
	if (!packed) return undefined
	if (editing.value) {
		return {
			gridColumn: `span ${packed.effectiveColumns}`,
			gridRow: `span ${packed.effectiveRows}`,
		}
	}
	return {
		gridColumn: `${packed.column} / span ${packed.effectiveColumns}`,
		gridRow: `${packed.row} / span ${packed.effectiveRows}`,
	}
}

function startWidgetDrag() {
	dragging.value = true
}

function finishWidgetDrag() {
	dragging.value = false
	const reordered = [...draggableWidgets.value]
	const unchanged = reordered.every(
		(widget, index) => widget.id === props.config.widgets[index]?.id,
	)
	if (!unchanged) emit('change', replaceHomeDashboardWidgets(props.config, reordered))
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
	const sizeOptions = HOME_WIDGET_SIZE_OPTIONS[widget.kind]
	return [
		...(sizeOptions.length > 1
			? [
					...sizeOptions.map((size) => ({
						id: `size-${size}`,
						icon: ExpandIcon,
						disabled: widget.size === size,
						action: () => resizeWidget(widget.id, size),
					})),
					{ divider: true },
				]
			: []),
		...(widget.target
			? [
					{
						id: 'replace',
						icon: RefreshCwIcon,
						action: () => replaceWidgetTarget(widget),
					},
					{ divider: true },
				]
			: []),
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
	<section class="home-dashboard p-6 pb-20" :class="{ 'is-dragging': dragging }">
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
				:list="draggableWidgets"
				item-key="id"
				tag="div"
				class="home-dashboard-grid"
				:class="{ 'is-editing': editing }"
				:style="{ gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))` }"
				handle=".home-widget-drag-handle"
				:disabled="!editing"
				:animation="220"
				:swap-threshold="0.65"
				:invert-swap="false"
				:empty-insert-threshold="24"
				:force-fallback="true"
				:fallback-on-body="true"
				:fallback-tolerance="3"
				ghost-class="home-widget-ghost"
				chosen-class="home-widget-chosen"
				drag-class="home-widget-drag"
				fallback-class="home-widget-fallback"
				data-onboarding-id="home-widget-grid"
				@start="startWidgetDrag"
				@end="finishWidgetDrag"
			>
				<template #item="{ element: widget, index }">
					<article class="home-widget" :data-widget-kind="widget.kind" :style="widgetStyle(widget)">
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
								:dashboard-size="effectiveSize(widget)"
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
								:dashboard-size="effectiveSize(widget)"
								dashboard
							/>
							<HomePinnedWorlds
								v-else-if="widget.kind === 'pinned-worlds'"
								:instances="instances"
								:dashboard-size="effectiveSize(widget)"
								dashboard
							/>
							<HomePinnedServers
								v-else-if="widget.kind === 'pinned-servers'"
								:instances="instances"
								:dashboard-size="effectiveSize(widget)"
								dashboard
							/>
							<HomeShortcutWidget
								v-else
								:placement="widget"
								:instances="instances"
								:dashboard-size="effectiveSize(widget)"
							/>
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
	min-width: 0;
	container-type: inline-size;
}

.home-dashboard-toolbar {
	position: sticky;
	top: 1rem;
	z-index: 30;
	display: flex;
	width: fit-content;
	margin-left: auto;
	margin-bottom: 1rem;
	gap: 0.25rem;
	padding: 0.25rem;
	border: 1px solid var(--surface-5);
	border-radius: 8px;
	background: var(--surface-2);
	box-shadow: var(--shadow-card);
}

.home-dashboard-grid {
	display: grid;
	grid-auto-rows: 10rem;
	align-items: stretch;
	gap: 1rem;
}

.home-dashboard-grid.is-editing {
	grid-auto-flow: dense;
}

.home-widget {
	--widget-accent: var(--color-brand);

	position: relative;
	display: flex;
	min-width: 0;
	min-height: 0;
	flex-direction: column;
	overflow: hidden;
	box-sizing: border-box;
	border: 1px solid var(--surface-5);
	border-radius: 8px;
	background: var(--surface-3);
	box-shadow: var(--shadow-card);
	transition:
		border-color 140ms ease,
		box-shadow 140ms ease,
		transform 140ms ease;
}

.home-widget::before {
	position: absolute;
	top: 0;
	left: 0;
	z-index: 2;
	width: 100%;
	height: 3px;
	background: var(--widget-accent);
	content: '';
	pointer-events: none;
}

.home-widget[data-widget-kind='recent'],
.home-widget[data-widget-kind='server'],
.home-widget[data-widget-kind='pinned-servers'] {
	--widget-accent: var(--color-blue);
}

.home-widget[data-widget-kind='calendar'] {
	--widget-accent: var(--color-orange);
}

.home-widget[data-widget-kind='instance'],
.home-widget[data-widget-kind='pinned-instances'] {
	--widget-accent: var(--color-green);
}

.home-widget[data-widget-kind='world'],
.home-widget[data-widget-kind='pinned-worlds'] {
	--widget-accent: var(--color-brand);
}

.home-dashboard-grid:not(.is-editing) .home-widget:hover {
	border-color: color-mix(in srgb, var(--widget-accent) 38%, var(--surface-5));
	box-shadow:
		var(--shadow-card),
		0 6px 18px color-mix(in srgb, var(--surface-5) 28%, transparent);
	transform: translateY(-1px);
}

.home-widget-edit-bar {
	position: absolute;
	top: 0.5rem;
	left: 0.5rem;
	z-index: 12;
	display: flex;
	height: 2.5rem;
	align-items: center;
	gap: 0.5rem;
	padding: 0 0.375rem;
	border: 1px solid var(--surface-5);
	border-radius: 7px;
	background: color-mix(in srgb, var(--surface-2) 94%, transparent);
	box-shadow: var(--shadow-button);
	backdrop-filter: blur(8px);
}

.home-widget-drag-handle {
	display: inline-flex;
	width: 2.25rem;
	height: 2.25rem;
	align-items: center;
	justify-content: center;
	padding: 0;
	border: 0;
	border-radius: 6px;
	background: transparent;
	color: var(--color-secondary);
	cursor: grab;
	touch-action: none;
	transition:
		background-color 100ms ease,
		color 100ms ease;
}

.home-widget-drag-handle:hover,
.home-widget-drag-handle:focus-visible {
	background: var(--color-button-bg);
	color: var(--color-contrast);
	outline: none;
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
	border: 2px dashed var(--widget-accent);
	background: color-mix(in srgb, var(--widget-accent) 8%, var(--surface-3));
	box-shadow: none;
	opacity: 0.55;
}

.home-widget-ghost > * {
	opacity: 0;
}

.home-widget-chosen {
	border-color: var(--widget-accent);
	box-shadow: 0 0 0 3px color-mix(in srgb, var(--widget-accent) 22%, transparent);
}

.home-widget-drag,
.home-widget-fallback {
	z-index: 1000 !important;
	border-color: var(--widget-accent);
	box-shadow:
		var(--shadow-card),
		0 16px 36px color-mix(in srgb, var(--surface-5) 48%, transparent);
	cursor: grabbing;
	opacity: 0.96;
	transform: rotate(0.5deg);
}

.home-dashboard-grid.is-editing .home-widget {
	border-style: dashed;
	box-shadow: none;
}

.home-dashboard-grid.is-editing .home-widget-content {
	pointer-events: none;
	opacity: 0.72;
}

.home-dashboard.is-dragging,
.home-dashboard.is-dragging * {
	user-select: none;
}

@media (prefers-reduced-motion: reduce) {
	.home-dashboard-grid {
		scroll-behavior: auto;
	}
}
</style>
