<script setup lang="ts">
import { GridIcon, RightArrowIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { useHomeDashboardRuntime } from '@/components/home/home-dashboard-runtime'
import HomeInstanceCard from '@/components/home/HomeInstanceCard.vue'
import type { HomeWidgetSize } from '@/components/home/home-dashboard'
import { set_pinned } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instances: GameInstance[]
	dashboard?: boolean
	dashboardSize?: HomeWidgetSize | null
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { runningInstanceIds } = useHomeDashboardRuntime()
const messages = defineMessages({
	pinnedInstances: {
		id: 'app.home.instances.pinned',
		defaultMessage: 'Pinned instances',
	},
	emptyPinned: {
		id: 'app.home.instances.pinned-empty',
		defaultMessage: 'Pin an instance from its card menu or the library to keep it here.',
	},
	viewAllInstances: {
		id: 'app.home.instances.view-all',
		defaultMessage: 'View all instances',
	},
})

const pinnedInstances = computed(() =>
	props.instances
		.filter((instance) => instance.pinned_at)
		.slice()
		.sort((a, b) => new Date(b.pinned_at ?? 0).getTime() - new Date(a.pinned_at ?? 0).getTime()),
)
const cardLayout = computed(() => {
	if (props.dashboardSize === '1x1') return 'spotlight' as const
	if (props.dashboardSize === '2x2') return 'tile' as const
	return 'row' as const
})

async function updatePinned(instance: GameInstance, pinned: boolean) {
	await set_pinned(instance.id, pinned).catch(handleError)
}
</script>

<template>
	<section class="home-pinned-instances" :data-size="dashboardSize">
		<div class="home-widget-heading">
			<h2>
				{{ formatMessage(messages.pinnedInstances) }}
			</h2>
			<ButtonStyled v-if="dashboardSize !== '1x1'" type="transparent" size="small" class="ml-auto">
				<router-link to="/library">
					<span v-if="dashboardSize === '2x2'">{{ formatMessage(messages.viewAllInstances) }}</span>
					<RightArrowIcon aria-hidden="true" />
				</router-link>
			</ButtonStyled>
		</div>
		<div v-if="pinnedInstances.length > 0" class="home-instance-list">
			<HomeInstanceCard
				v-for="instance in pinnedInstances"
				:key="instance.id"
				:instance="instance"
				:pinned="true"
				:layout="cardLayout"
				:playing="runningInstanceIds.includes(instance.id)"
				@pinned-change="updatePinned"
			/>
		</div>
		<div v-else class="home-widget-empty">
			<GridIcon aria-hidden="true" />
			<span>{{ formatMessage(messages.emptyPinned) }}</span>
		</div>
	</section>
</template>

<style scoped>
.home-pinned-instances {
	display: flex;
	min-width: 0;
	min-height: 0;
	height: 100%;
	flex-direction: column;
	gap: 0.75rem;
}

.home-widget-heading {
	display: flex;
	min-width: 0;
	height: 2rem;
	flex: 0 0 auto;
	align-items: center;
	gap: 0.5rem;
}

.home-widget-heading h2 {
	min-width: 0;
	overflow: hidden;
	margin: 0;
	color: var(--color-contrast);
	font-size: 1rem;
	font-weight: 700;
	letter-spacing: 0;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-instance-list {
	display: grid;
	min-width: 0;
	min-height: 0;
	flex: 1;
	grid-auto-rows: max-content;
	gap: 0.25rem;
	overflow-x: hidden;
	overflow-y: auto;
	padding-right: 0.25rem;
}

.home-pinned-instances[data-size='2x1'] .home-instance-list,
.home-pinned-instances[data-size='2x2'] .home-instance-list {
	grid-template-columns: repeat(2, minmax(0, 1fr));
	column-gap: 0.5rem;
}

.home-pinned-instances[data-size='1x1'] {
	gap: 0.375rem;
}

.home-pinned-instances[data-size='1x1'] .home-widget-heading {
	height: 1.5rem;
}

.home-widget-empty {
	display: flex;
	max-width: 22rem;
	margin: auto;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
	color: var(--color-secondary);
	font-size: 0.8125rem;
	line-height: 1.4;
	text-align: center;
}

.home-widget-empty svg {
	width: 1.5rem;
	height: 1.5rem;
	opacity: 0.7;
}
</style>
