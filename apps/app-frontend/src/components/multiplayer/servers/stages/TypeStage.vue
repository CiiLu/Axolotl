<script setup lang="ts">
import { listServerTypes, type ServerTypeId } from '@modrinth/server'
import { defineMessages, DropdownSelect, Toggle, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { injectCreateServerFlow } from '../create-server-flow'
import { SERVER_TYPE_META } from '../server-type'

const { formatMessage } = useVIntl()
const ctx = injectCreateServerFlow()

const messages = defineMessages({
	heading: { id: 'app.servers.wizard.type-heading', defaultMessage: 'Choose a server type' },
	gameVersion: { id: 'app.servers.wizard.game-version', defaultMessage: 'Game version' },
	loaderVersion: { id: 'app.servers.wizard.loader-version', defaultMessage: 'Loader version' },
	showSnapshots: { id: 'app.servers.wizard.show-snapshots', defaultMessage: 'Show snapshots' },
	comingSoon: {
		id: 'app.servers.wizard.type-coming-soon',
		defaultMessage: 'Coming soon',
	},
})

const serverTypeOptions = listServerTypes()

const loaderVersionOptions = computed(() => ctx.loaderVersions.value.map((loader) => loader.id))

function selectType(typeId: string) {
	ctx.serverType.value = typeId as ServerTypeId
	void ctx.loadLoaderVersions()
}

function selectGameVersion(version: string) {
	ctx.selectedGameVersion.value = version
	void ctx.loadLoaderVersions()
}
</script>

<template>
	<div class="flex flex-col gap-5">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.heading) }}
			</h2>
		</div>

		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 md:grid-cols-3">
			<button
				v-for="type in serverTypeOptions"
				:key="type.id"
				type="button"
				class="flex flex-col gap-3 rounded-xl border border-solid p-4 text-left transition-colors"
				:class="
					ctx.serverType.value === type.id
						? 'border-brand bg-brand-highlight'
						: 'border-surface-4 bg-surface-2 hover:border-surface-5'
				"
				@click="selectType(type.id)"
			>
				<div class="flex items-start justify-between gap-2">
					<span
						class="flex size-10 shrink-0 items-center justify-center rounded-xl text-sm font-bold"
						:style="`--_color: ${SERVER_TYPE_META[type.id].colorVar}`"
						:class="[
							'text-[--_color,var(--color-brand)]',
							'bg-[color-mix(in_srgb,var(--_color)_14%,transparent)]',
						]"
					>
						{{ SERVER_TYPE_META[type.id].monogram }}
					</span>
					<span
						v-if="type.installMode === 'installer'"
						class="rounded-full border border-solid border-surface-5 bg-surface-3 px-2 py-0.5 text-[11px] font-medium leading-none text-secondary"
					>
						{{ formatMessage(messages.comingSoon) }}
					</span>
				</div>
				<div class="min-w-0">
					<span class="block truncate font-semibold text-contrast">{{ type.label }}</span>
					<span v-if="type.needsLoaderVersion" class="mt-0.5 block text-xs text-secondary">
						{{ formatMessage(messages.loaderVersion) }}
					</span>
				</div>
			</button>
		</div>

		<div class="grid gap-4 md:grid-cols-2">
			<div class="flex min-w-0 flex-col gap-2">
				<span class="font-semibold text-contrast">
					{{ formatMessage(messages.gameVersion) }}
				</span>
				<DropdownSelect
					:model-value="ctx.selectedGameVersion.value"
					:options="ctx.availableGameVersions.value"
					:name="formatMessage(messages.gameVersion)"
					@update:model-value="selectGameVersion"
				/>
			</div>

			<div v-if="ctx.needsLoaderVersion.value" class="flex min-w-0 flex-col gap-2">
				<span class="font-semibold text-contrast">
					{{ formatMessage(messages.loaderVersion) }}
				</span>
				<DropdownSelect
					:model-value="ctx.selectedLoaderVersion.value"
					:options="loaderVersionOptions"
					:name="formatMessage(messages.loaderVersion)"
					@update:model-value="(value) => (ctx.selectedLoaderVersion.value = value)"
				/>
			</div>
		</div>

		<div class="flex items-center justify-between gap-4 rounded-xl bg-surface-2 px-4 py-3">
			<span class="text-sm text-secondary">
				{{ formatMessage(messages.showSnapshots) }}
			</span>
			<Toggle
				id="wizard-show-snapshots"
				:model-value="ctx.showSnapshots.value"
				small
				@update:model-value="
					(value) => {
						ctx.showSnapshots.value = value
						void ctx.loadVersions()
					}
				"
			/>
		</div>
	</div>
</template>
