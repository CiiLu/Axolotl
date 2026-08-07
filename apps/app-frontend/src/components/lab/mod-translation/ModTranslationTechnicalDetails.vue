<script setup lang="ts">
import { ChevronDownIcon, ChevronUpIcon } from '@modrinth/assets'
import { computed, ref } from 'vue'

import type { ModTranslationJob } from '@/lab/mod-translation/types.ts'

const props = defineProps<{ job: ModTranslationJob }>()
const open = ref(false)
const diagnostic = computed(() => ({
	taskId: props.job.taskId,
	inputHash: props.job.inputHash,
	lastSequence: props.job.lastSequence,
	status: props.job.status,
	error: props.job.error,
	events: props.job.events,
}))

function copy() {
	void navigator.clipboard.writeText(JSON.stringify(diagnostic.value, null, 2))
}
</script>

<template>
	<section class="technical">
		<div class="head">
			<button :aria-expanded="open" @click="open = !open">
				技术详情 <ChevronUpIcon v-if="open" /><ChevronDownIcon v-else />
			</button>
			<button @click="copy">复制诊断信息</button>
		</div>
		<pre v-if="open">{{ JSON.stringify(diagnostic, null, 2) }}</pre>
	</section>
</template>

<style scoped>
.technical { display: flex; flex-direction: column; gap: 0.4rem; }
.head { display: flex; justify-content: space-between; gap: 0.5rem; }
.head button { display: inline-flex; align-items: center; gap: 0.3rem; border: 0; background: transparent; padding: 0.2rem; color: var(--color-text-secondary); font-size: 0.68rem; }
.head button:focus-visible { outline: 2px solid var(--color-brand); outline-offset: 2px; }
.head :deep(svg) { width: 0.75rem; height: 0.75rem; }
pre { max-height: 18rem; overflow: auto; margin: 0; border-radius: var(--radius-sm); background: var(--surface-1); padding: 0.65rem; font-size: 0.62rem; white-space: pre-wrap; }
</style>
