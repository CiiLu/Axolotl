<script setup lang="ts">
import { cn } from '~/lib/utils'

const props = withDefaults(
	defineProps<{
		variant?: 'default' | 'secondary' | 'success' | 'warning' | 'destructive'
		class?: string
	}>(),
	{ variant: 'secondary', class: undefined },
)

const variants = {
	default: 'border-transparent bg-primary text-primary-foreground',
	secondary: 'border-transparent bg-secondary text-secondary-foreground',
	success: 'border-transparent bg-emerald-500/15 text-emerald-700 dark:text-emerald-300',
	warning: 'border-transparent bg-amber-500/15 text-amber-700 dark:text-amber-300',
	destructive: 'border-transparent bg-red-500/15 text-red-700 dark:text-red-300',
}
</script>

<template>
	<span
		:class="
			cn(
				'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors',
				variants[variant],
				props.class,
			)
		"
	>
		<slot />
	</span>
</template>
