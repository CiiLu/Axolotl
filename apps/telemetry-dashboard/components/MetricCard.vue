<script setup lang="ts">
import type { Component } from 'vue'

import Card from '~/components/ui/Card.vue'
import { formatNumber } from '~/utils/format'

const props = withDefaults(
	defineProps<{
		title: string
		value: number
		detail: string
		icon: Component
		tone?: 'indigo' | 'emerald' | 'cyan' | 'sky' | 'amber' | 'rose' | 'violet' | 'neutral'
	}>(),
	{ tone: 'neutral' },
)

const tones = {
	indigo: 'bg-indigo-500/12 text-indigo-700 dark:text-indigo-300',
	emerald: 'bg-emerald-500/12 text-emerald-700 dark:text-emerald-300',
	cyan: 'bg-cyan-500/12 text-cyan-700 dark:text-cyan-300',
	sky: 'bg-sky-500/12 text-sky-700 dark:text-sky-300',
	amber: 'bg-amber-500/12 text-amber-700 dark:text-amber-300',
	rose: 'bg-rose-500/12 text-rose-700 dark:text-rose-300',
	violet: 'bg-violet-500/12 text-violet-700 dark:text-violet-300',
	neutral: 'bg-surface-3 text-muted-foreground',
}
</script>

<template>
	<Card class="min-w-0 p-5">
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<p class="truncate text-sm font-medium text-muted-foreground" :title="title">{{ title }}</p>
				<p class="mt-2 text-2xl font-semibold tabular-nums tracking-tight">
					{{ formatNumber(value) }}
				</p>
			</div>
			<div
				class="flex size-9 shrink-0 items-center justify-center rounded-lg"
				:class="tones[props.tone]"
			>
				<component :is="icon" class="size-[18px]" />
			</div>
		</div>
		<p class="mt-2 truncate text-xs text-muted-foreground" :title="detail">{{ detail }}</p>
	</Card>
</template>
