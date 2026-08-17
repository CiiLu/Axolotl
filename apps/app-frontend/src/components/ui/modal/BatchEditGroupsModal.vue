<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" fade="standard" max-width="500px">
		<p class="m-0 text-secondary">
			{{ formatMessage(messages.description, { count: instanceIds.length }) }}
		</p>

		<div class="flex flex-col gap-3 mt-4">
			<div class="flex flex-col gap-1">
				<Checkbox
					v-for="group in availableGroups"
					:key="group"
					:model-value="selectedGroups.includes(group)"
					:label="group"
					@click="toggleGroup(group)"
				/>
			</div>

			<div class="flex gap-2 items-center">
				<StyledInput
					v-model="newGroupInput"
					:placeholder="formatMessage(messages.enterGroupName)"
					class="w-full max-w-[300px]"
					@submit="addNewGroup"
				/>
				<ButtonStyled>
					<button class="w-fit !shadow-none" @click="addNewGroup">
						<PlusIcon /> {{ formatMessage(messages.createGroup) }}
					</button>
				</ButtonStyled>
			</div>
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<ButtonStyled type="outlined">
					<button @click="modal?.hide()">
						<XIcon />
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button @click="confirm">
						<CheckIcon />
						{{ formatMessage(messages.applyButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, PlusIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Checkbox,
	commonMessages,
	defineMessages,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { edit, list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const { formatMessage } = useVIntl()

const props = defineProps<{
	instanceIds: string[]
}>()

const emit = defineEmits<{
	(e: 'applied'): void
}>()

const messages = defineMessages({
	header: {
		id: 'app.instances.batch-edit-groups.header',
		defaultMessage: 'Edit groups',
	},
	description: {
		id: 'app.instances.batch-edit-groups.description',
		defaultMessage: 'Select groups to apply to {count} instance(s).',
	},
	enterGroupName: {
		id: 'app.instances.batch-edit-groups.enter-group-name',
		defaultMessage: 'Enter group name',
	},
	createGroup: {
		id: 'app.instances.batch-edit-groups.create-group',
		defaultMessage: 'Create new group',
	},
	applyButton: {
		id: 'app.instances.batch-edit-groups.apply',
		defaultMessage: 'Apply',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const selectedGroups = ref<string[]>([])
const newGroupInput = ref('')
const allInstances = ref<GameInstance[]>([])

const availableGroups = computed(() => {
	const groups = new Set<string>()
	for (const instance of allInstances.value) {
		for (const group of instance.groups) {
			groups.add(group)
		}
	}
	return [...groups]
})

function show() {
	selectedGroups.value = []
	newGroupInput.value = ''
	list().then((instances) => {
		allInstances.value = instances as GameInstance[]
	})
	modal.value?.show()
}

function toggleGroup(group: string) {
	if (selectedGroups.value.includes(group)) {
		selectedGroups.value = selectedGroups.value.filter((x) => x !== group)
	} else {
		selectedGroups.value.push(group)
	}
}

function addNewGroup() {
	const text = newGroupInput.value.trim()
	if (text.length > 0) {
		const groupName = text.substring(0, 32)
		selectedGroups.value.push(groupName)
		// Add a dummy instance with this group so it shows in availableGroups
		allInstances.value.push({ groups: [groupName] } as GameInstance)
		newGroupInput.value = ''
	}
}

async function confirm() {
	modal.value?.hide()

	const groups = selectedGroups.value
		.map((x) => x.trim().substring(0, 32))
		.filter((x) => x.length > 0)

	for (const instanceId of props.instanceIds) {
		await edit(instanceId, { groups }).catch(() => {})
	}

	emit('applied')
}

defineExpose({
	show,
})
</script>
