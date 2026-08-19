<script setup lang="ts">
import { SaveIcon, SpinnerIcon, TrashIcon } from '@modrinth/assets'
import { requiredJavaMajorVersion } from '@modrinth/server'
import {
	ButtonStyled,
	Card,
	ConfirmModal,
	defineMessages,
	DropdownSelect,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { onMounted, ref, useTemplateRef } from 'vue'
import type { ComponentExposed } from 'vue-component-type-helpers'

import ServerPropertiesEditor from '@/components/multiplayer/servers/ServerPropertiesEditor.vue'
import { type ServerView, useServers } from '@/composables/useServers'
import { find_filtered_jres } from '@/helpers/jre'
import { servers as serversApi } from '@/helpers/servers'

const props = defineProps<{
	server: ServerView
}>()

const emit = defineEmits<{
	deleted: []
}>()

const { formatMessage } = useVIntl()
const messages = defineMessages({
	general: { id: 'app.servers.settings.general', defaultMessage: 'General' },
	name: { id: 'app.servers.settings.name', defaultMessage: 'Server name' },
	java: { id: 'app.servers.settings.java', defaultMessage: 'Java' },
	javaNone: { id: 'app.servers.settings.java-none', defaultMessage: 'System default' },
	memory: { id: 'app.servers.settings.memory', defaultMessage: 'Memory (MB)' },
	jvmArgs: { id: 'app.servers.settings.jvm-args', defaultMessage: 'JVM arguments' },
	jvmArgsHint: {
		id: 'app.servers.settings.jvm-args-hint',
		defaultMessage: 'Space-separated arguments, e.g. -XX:+UseG1GC',
	},
	save: { id: 'app.servers.settings.save', defaultMessage: 'Save changes' },
	saved: { id: 'app.servers.settings.saved', defaultMessage: 'Server settings saved' },
	deleteTitle: { id: 'app.servers.settings.delete', defaultMessage: 'Delete server' },
	deleteHint: {
		id: 'app.servers.settings.delete-hint',
		defaultMessage: 'Permanently remove this server and all of its files.',
	},
	deleteConfirm: {
		id: 'app.servers.settings.delete-confirm',
		defaultMessage: 'Delete {name} and all of its files? This cannot be undone.',
	},
	deleteProceed: { id: 'app.servers.settings.delete-proceed', defaultMessage: 'Delete' },
	configFiles: { id: 'app.servers.settings.config', defaultMessage: 'Configuration' },
})

const { deleteServer } = useServers()
const { addNotification, handleError } = injectNotificationManager()

const name = ref(props.server.name)
const javaPath = ref(props.server.javaPath ?? '')
const memoryMb = ref(props.server.memoryMb ?? 2048)
const jvmArgsText = ref((props.server.jvmArgs ?? []).join(' '))
const javaOptions = ref<string[]>([])
const isSaving = ref(false)
const deleteModal = useTemplateRef<ComponentExposed<typeof ConfirmModal>>('deleteModal')

onMounted(async () => {
	try {
		const major = requiredJavaMajorVersion(props.server.gameVersion)
		const javas = (await find_filtered_jres(major)) as Array<{ path: string; version: string }>
		javaOptions.value = ['', ...javas.map((java) => java.path)]
	} catch {
		javaOptions.value = ['']
	}
})

function javaOptionLabel(value: string) {
	return value === '' ? formatMessage(messages.javaNone) : value
}

async function save() {
	isSaving.value = true
	try {
		const jvmArgs = jvmArgsText.value.trim().split(/\s+/).filter(Boolean)
		await serversApi.updateSettings(props.server.id, {
			name: name.value.trim(),
			javaPath: javaPath.value,
			memoryMb: memoryMb.value,
			jvmArgs,
		})
		addNotification({ type: 'success', title: formatMessage(messages.saved) })
	} catch (error) {
		handleError(error)
	} finally {
		isSaving.value = false
	}
}

async function confirmDelete() {
	const ok = await deleteServer(props.server.id)
	if (ok) emit('deleted')
}
</script>

<template>
	<div class="flex flex-col gap-6">
		<Card data-onboarding-id="server-settings" class="!m-0 max-w-3xl">
			<div class="flex flex-col gap-4">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.general) }}
				</h3>

				<label class="flex min-w-0 flex-col gap-2" for="server-settings-name">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.name) }}</span>
					<StyledInput id="server-settings-name" v-model="name" />
				</label>

				<div class="grid gap-4 md:grid-cols-2">
					<div class="flex min-w-0 flex-col gap-2">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.java) }}</span>
						<DropdownSelect
							v-model="javaPath"
							class="!w-full"
							:options="javaOptions"
							:display-name="javaOptionLabel"
							:name="formatMessage(messages.java)"
						/>
					</div>

					<label class="flex min-w-0 flex-col gap-2" for="server-settings-memory">
						<span class="font-semibold text-contrast">{{ formatMessage(messages.memory) }}</span>
						<StyledInput id="server-settings-memory" v-model="memoryMb" inputmode="numeric" />
					</label>
				</div>

				<label class="flex min-w-0 flex-col gap-2" for="server-settings-jvm">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.jvmArgs) }}</span>
					<StyledInput id="server-settings-jvm" v-model="jvmArgsText" />
					<span class="text-xs text-secondary">{{ formatMessage(messages.jvmArgsHint) }}</span>
				</label>

				<div class="flex justify-end">
					<ButtonStyled color="brand">
						<button type="button" :disabled="isSaving" @click="save">
							<SpinnerIcon v-if="isSaving" class="animate-spin" />
							<SaveIcon v-else />
							{{ formatMessage(messages.save) }}
						</button>
					</ButtonStyled>
				</div>
			</div>
		</Card>

		<Card class="!m-0 max-w-3xl">
			<ServerPropertiesEditor :server-id="server.id" />
		</Card>

		<Card class="!m-0 max-w-3xl">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div class="flex min-w-0 items-start gap-3">
					<div
						class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-red-highlight text-red"
					>
						<TrashIcon class="size-4" />
					</div>
					<div class="min-w-0">
						<h3 class="m-0 text-base font-semibold text-contrast">
							{{ formatMessage(messages.deleteTitle) }}
						</h3>
						<p class="mb-0 mt-1 text-sm text-secondary">
							{{ formatMessage(messages.deleteHint) }}
						</p>
					</div>
				</div>
				<ButtonStyled color="danger" type="outlined">
					<button type="button" :disabled="server.running" @click="deleteModal?.show()">
						<TrashIcon />
						{{ formatMessage(messages.deleteTitle) }}
					</button>
				</ButtonStyled>
			</div>
		</Card>

		<ConfirmModal
			ref="deleteModal"
			:title="formatMessage(messages.deleteTitle)"
			:description="formatMessage(messages.deleteConfirm, { name: server.name })"
			:proceed-label="formatMessage(messages.deleteProceed)"
			@proceed="confirmDelete"
		/>
	</div>
</template>
