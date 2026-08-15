<script setup lang="ts">
import type { Component } from 'vue'

import Card from '~/components/ui/Card.vue'
import { formatNumber } from '~/utils/format'

defineProps<{
	title: string
	value: number
	detail: string
	icon: Component
}>()
</script>

<template>
	<Card class="min-w-0 p-5">
		<div class="flex items-center justify-between gap-3">
			<p class="truncate text-sm font-medium text-muted-foreground" :title="title">{{ title }}</p>
			<component :is="icon" class="size-4 shrink-0 text-muted-foreground" />
		</div>
		<p class="mt-2 text-2xl font-semibold tabular-nums tracking-tight">
			{{ formatNumber(value) }}
		</p>
		<p class="mt-1 truncate text-xs text-muted-foreground" :title="detail">{{ detail }}</p>
	</Card>
</template>
