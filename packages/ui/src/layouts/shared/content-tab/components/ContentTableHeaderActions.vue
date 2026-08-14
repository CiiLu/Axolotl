<script setup lang="ts">
import {
	ArrowDownAZIcon,
	ArrowUpZAIcon,
	ClockArrowDownIcon,
	ClockArrowUpIcon,
	DownloadIcon,
} from '@modrinth/assets'

import ButtonStyled from '#ui/components/base/ButtonStyled.vue'

const props = withDefaults(
	defineProps<{
		sortMode: string
		sortLabel: string
		hasBulkUpdateSupport?: boolean
		hasOutdatedProjects?: boolean
		bulkUpdateTooltip?: string
		isBulkOperating?: boolean
	}>(),
	{
		hasBulkUpdateSupport: false,
		hasOutdatedProjects: false,
		bulkUpdateTooltip: undefined,
		isBulkOperating: false,
	},
)

const emit = defineEmits<{
	sort: []
	updateAll: []
}>()
</script>

<template>
	<div class="flex items-center justify-end gap-2">
		<ButtonStyled circular type="transparent">
			<button v-tooltip="props.sortLabel" :aria-label="props.sortLabel" @click="emit('sort')">
				<ArrowUpZAIcon v-if="props.sortMode === 'alphabetical-desc'" /><ClockArrowDownIcon
					v-else-if="props.sortMode === 'date-added-newest'"
				/><ClockArrowUpIcon v-else-if="props.sortMode === 'date-added-oldest'" /><ArrowDownAZIcon
					v-else
				/>
			</button>
		</ButtonStyled>

		<ButtonStyled
			v-if="props.hasBulkUpdateSupport && props.hasOutdatedProjects"
			circular
			color="green"
			type="transparent"
			color-fill="text"
			hover-color-fill="background"
		>
			<button
				v-tooltip="props.bulkUpdateTooltip"
				:disabled="props.isBulkOperating"
				@click="emit('updateAll')"
			>
				<DownloadIcon />
			</button>
		</ButtonStyled>
	</div>
</template>
