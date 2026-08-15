<script setup lang="ts">
import type { ButtonHTMLAttributes } from 'vue'

import { cn } from '~/lib/utils'

const props = withDefaults(
	defineProps<{
		variant?: 'default' | 'secondary' | 'ghost' | 'outline' | 'destructive'
		size?: 'default' | 'sm' | 'icon'
		class?: ButtonHTMLAttributes['class']
	}>(),
	{ variant: 'default', size: 'default', class: undefined },
)

const variants = {
	default: 'bg-primary text-primary-foreground shadow-sm hover:bg-primary/90',
	secondary: 'bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80',
	ghost: 'text-foreground hover:bg-accent hover:text-accent-foreground',
	outline:
		'border-input bg-surface-2 text-foreground shadow-sm hover:bg-accent hover:text-accent-foreground',
	destructive: 'bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90',
}

const sizes = {
	default: 'h-9 px-3.5',
	sm: 'h-8 px-2.5 text-xs',
	icon: 'size-9 shrink-0 p-0',
}
</script>

<template>
	<button
		:data-variant="variant"
		:class="
			cn(
				'inline-flex cursor-pointer items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background active:brightness-95 disabled:pointer-events-auto disabled:cursor-not-allowed disabled:opacity-50',
				variants[props.variant],
				sizes[props.size],
				props.class,
			)
		"
	>
		<slot />
	</button>
</template>
