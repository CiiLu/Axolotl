<script setup lang="ts">
import {
	CalendarIcon,
	GameIcon,
	GridIcon,
	HistoryIcon,
	SearchIcon,
	ServerIcon,
	UserIcon,
} from '@modrinth/assets'
import { defineMessages, NewModal, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, nextTick, ref } from 'vue'

import type { HomeWidgetKind, HomeWidgetPlacement } from '@/components/home/home-dashboard'
import { HOME_WIDGET_DEFAULT_SIZE } from '@/components/home/home-dashboard'
import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import type { GameInstance } from '@/helpers/types'
import type { World } from '@/helpers/worlds'

const props = defineProps<{
	instances: GameInstance[]
}>()

const emit = defineEmits<{
	add: [widget: HomeWidgetPlacement]
}>()

const { formatMessage, locale } = useVIntl()
const runtime = useHomeDashboardRuntime()
const modal = ref<InstanceType<typeof NewModal>>()
const searchInput = ref<InstanceType<typeof StyledInput>>()
const searchQuery = ref('')
const selectedKind = ref<HomeWidgetKind | null>(null)
const selectedInstance = ref<GameInstance | null>(null)
const worlds = ref<World[]>([])
const loadingWorlds = ref(false)

const messages = defineMessages({
	title: { id: 'app.home.widgets.add-title', defaultMessage: 'Add widget' },
	search: { id: 'app.home.widgets.search', defaultMessage: 'Search' },
	back: { id: 'app.home.widgets.back', defaultMessage: 'Back' },
	noResults: { id: 'app.home.widgets.no-results', defaultMessage: 'No matching items' },
	loading: { id: 'app.home.widgets.loading', defaultMessage: 'Loading...' },
	greeting: { id: 'app.home.widgets.greeting', defaultMessage: 'Greeting' },
	recent: { id: 'app.home.widgets.recent', defaultMessage: 'Recently played' },
	calendar: { id: 'app.home.widgets.calendar', defaultMessage: 'Calendar' },
	pinnedInstances: { id: 'app.home.widgets.pinned-instances', defaultMessage: 'Pinned instances' },
	pinnedWorlds: { id: 'app.home.widgets.pinned-worlds', defaultMessage: 'Pinned worlds' },
	pinnedServers: { id: 'app.home.widgets.pinned-servers', defaultMessage: 'Pinned servers' },
	instance: { id: 'app.home.widgets.instance', defaultMessage: 'Instance shortcut' },
	world: { id: 'app.home.widgets.world', defaultMessage: 'World shortcut' },
	server: { id: 'app.home.widgets.server', defaultMessage: 'Server shortcut' },
	chooseInstance: {
		id: 'app.home.widgets.choose-instance',
		defaultMessage: 'Choose an instance',
	},
	chooseWorld: { id: 'app.home.widgets.choose-world', defaultMessage: 'Choose a world' },
	chooseServer: { id: 'app.home.widgets.choose-server', defaultMessage: 'Choose a server' },
})

const catalog = computed(() => [
	{ kind: 'greeting' as const, label: formatMessage(messages.greeting), icon: UserIcon },
	{ kind: 'recent' as const, label: formatMessage(messages.recent), icon: HistoryIcon },
	{ kind: 'calendar' as const, label: formatMessage(messages.calendar), icon: CalendarIcon },
	{
		kind: 'pinned-instances' as const,
		label: formatMessage(messages.pinnedInstances),
		icon: GridIcon,
	},
	{ kind: 'pinned-worlds' as const, label: formatMessage(messages.pinnedWorlds), icon: GameIcon },
	{
		kind: 'pinned-servers' as const,
		label: formatMessage(messages.pinnedServers),
		icon: ServerIcon,
	},
	{ kind: 'instance' as const, label: formatMessage(messages.instance), icon: GridIcon },
	{ kind: 'world' as const, label: formatMessage(messages.world), icon: GameIcon },
	{ kind: 'server' as const, label: formatMessage(messages.server), icon: ServerIcon },
])

const filteredInstances = computed(() => {
	const query = searchQuery.value.trim().toLocaleLowerCase(locale.value)
	return props.instances.filter((instance) =>
		query ? instance.name.toLocaleLowerCase(locale.value).includes(query) : true,
	)
})

const filteredWorlds = computed(() => {
	const type = selectedKind.value === 'server' ? 'server' : 'singleplayer'
	const query = searchQuery.value.trim().toLocaleLowerCase(locale.value)
	return worlds.value.filter(
		(world) =>
			world.type === type && (!query || world.name.toLocaleLowerCase(locale.value).includes(query)),
	)
})

const pickerTitle = computed(() => {
	if (!selectedKind.value) return formatMessage(messages.title)
	if (!selectedInstance.value) return formatMessage(messages.chooseInstance)
	return formatMessage(
		selectedKind.value === 'server' ? messages.chooseServer : messages.chooseWorld,
	)
})

function show(kind: HomeWidgetKind | null = null) {
	selectedKind.value = kind
	selectedInstance.value = null
	worlds.value = []
	searchQuery.value = ''
	modal.value?.show()
	if (kind) void nextTick(() => searchInput.value?.focus())
}

function addWidget(widget: HomeWidgetPlacement) {
	emit('add', widget)
	modal.value?.hide()
}

function chooseKind(kind: HomeWidgetKind) {
	if (kind !== 'instance' && kind !== 'world' && kind !== 'server') {
		addWidget({ id: crypto.randomUUID(), kind, size: HOME_WIDGET_DEFAULT_SIZE[kind] })
		return
	}
	selectedKind.value = kind
	searchQuery.value = ''
	void nextTick(() => searchInput.value?.focus())
}

async function chooseInstance(instance: GameInstance) {
	if (selectedKind.value === 'instance') {
		addWidget({
			id: crypto.randomUUID(),
			kind: 'instance',
			size: HOME_WIDGET_DEFAULT_SIZE.instance,
			target: { instanceId: instance.id, fallbackLabel: instance.name },
		})
		return
	}

	selectedInstance.value = instance
	searchQuery.value = ''
	loadingWorlds.value = true
	worlds.value = await runtime.getInstanceWorlds(instance.id)
	loadingWorlds.value = false
}

function chooseWorld(world: World) {
	if (!selectedInstance.value || (world.type !== 'server' && world.type !== 'singleplayer')) return
	const kind = world.type === 'server' ? 'server' : 'world'
	addWidget({
		id: crypto.randomUUID(),
		kind,
		size: HOME_WIDGET_DEFAULT_SIZE[kind],
		target: {
			instanceId: selectedInstance.value.id,
			...(world.type === 'server' ? { address: world.address } : { path: world.path }),
			fallbackLabel: world.name,
		},
	})
}

function goBack() {
	searchQuery.value = ''
	if (selectedInstance.value) {
		selectedInstance.value = null
		worlds.value = []
	} else {
		selectedKind.value = null
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal
		ref="modal"
		:header="pickerTitle"
		max-width="640px"
		width="min(640px, calc(100vw - 2rem))"
		scrollable
		max-content-height="min(38rem, 72vh)"
	>
		<div class="flex min-w-0 flex-col gap-4">
			<button
				v-if="selectedKind"
				type="button"
				class="w-fit cursor-pointer border-0 bg-transparent p-0 text-sm font-semibold text-brand hover:underline"
				@click="goBack"
			>
				{{ formatMessage(messages.back) }}
			</button>

			<div v-if="!selectedKind" class="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-2">
				<button
					v-for="item in catalog"
					:key="item.kind"
					type="button"
					class="flex min-h-24 cursor-pointer flex-col items-start justify-between gap-4 rounded-lg border border-solid border-divider bg-bg-raised p-4 text-left text-primary transition-colors hover:bg-button-bg focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-brand-shadow"
					@click="chooseKind(item.kind)"
				>
					<component :is="item.icon" class="size-5 text-brand" aria-hidden="true" />
					<span class="font-semibold text-contrast">{{ item.label }}</span>
				</button>
			</div>

			<template v-else>
				<StyledInput
					ref="searchInput"
					v-model="searchQuery"
					type="search"
					:icon="SearchIcon"
					:placeholder="formatMessage(messages.search)"
					wrapper-class="w-full"
					clearable
				/>
				<p v-if="loadingWorlds" class="m-0 py-8 text-center text-sm text-secondary">
					{{ formatMessage(messages.loading) }}
				</p>
				<ul v-else class="m-0 flex list-none flex-col gap-1 p-0">
					<li
						v-for="item in selectedInstance ? filteredWorlds : filteredInstances"
						:key="'id' in item ? item.id : item.type === 'server' ? item.address : item.path"
					>
						<button
							type="button"
							class="flex min-h-14 w-full cursor-pointer items-center gap-3 rounded-lg border-0 bg-transparent px-3 py-2 text-left hover:bg-button-bg"
							@click="
								selectedInstance ? chooseWorld(item as World) : chooseInstance(item as GameInstance)
							"
						>
							<InstanceIcon
								v-if="'id' in item"
								class="size-9 shrink-0"
								:icon-path="item.icon_path"
								:instance-id="item.id"
							/>
							<ServerIcon v-else-if="item.type === 'server'" class="size-5 shrink-0" />
							<GameIcon v-else class="size-5 shrink-0" />
							<span class="min-w-0 flex-1 truncate font-semibold text-contrast">{{
								item.name
							}}</span>
						</button>
					</li>
				</ul>
				<p
					v-if="
						!loadingWorlds &&
						(selectedInstance ? filteredWorlds.length === 0 : filteredInstances.length === 0)
					"
					class="m-0 py-8 text-center text-sm text-secondary"
				>
					{{ formatMessage(messages.noResults) }}
				</p>
			</template>
		</div>
	</NewModal>
</template>
