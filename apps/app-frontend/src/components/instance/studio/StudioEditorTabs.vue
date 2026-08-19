<script setup lang="ts">
import { FileCodeIcon, XIcon } from '@modrinth/assets'
import { commonMessages, useVIntl } from '@modrinth/ui'

import type { StudioDocument } from './useStudioDocuments'

defineProps<{
	documents: StudioDocument[]
	activePath: string
}>()

const emit = defineEmits<{
	activate: [path: string]
	close: [path: string]
}>()

const { formatMessage } = useVIntl()

function handleAuxClick(event: MouseEvent, path: string) {
	if (event.button !== 1) return
	event.preventDefault()
	emit('close', path)
}
</script>

<template>
	<div
		v-for="document in documents"
		:key="document.path"
		role="tab"
		tabindex="0"
		:aria-selected="document.path === activePath"
		class="flex h-full max-w-[14rem] min-w-[8rem] shrink-0 items-center gap-2 border-0 border-r border-solid border-surface-4 px-3 text-left text-sm text-secondary hover:bg-surface-2"
		:class="{ 'bg-surface-2 !text-contrast': document.path === activePath }"
		@click="emit('activate', document.path)"
		@auxclick="handleAuxClick($event, document.path)"
		@keydown.enter="emit('activate', document.path)"
		@keydown.space.prevent="emit('activate', document.path)"
	>
		<FileCodeIcon class="size-4 shrink-0 text-secondary" />
		<span class="min-w-0 flex-1 truncate">{{ document.name }}</span>
		<span
			v-if="document.content !== document.savedContent"
			class="size-2 shrink-0 rounded-full bg-brand"
		/>
		<button
			type="button"
			:aria-label="formatMessage(commonMessages.closeButton)"
			class="flex size-5 shrink-0 items-center justify-center rounded border-0 bg-transparent p-0 text-secondary hover:bg-surface-4 hover:text-contrast"
			@pointerdown.stop
			@click.stop.prevent="emit('close', document.path)"
		>
			<XIcon class="size-3.5" />
		</button>
	</div>
</template>
