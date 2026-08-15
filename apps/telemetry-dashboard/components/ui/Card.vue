<script setup lang="ts">
import type { HTMLAttributes } from 'vue'

import { cn } from '~/lib/utils'

const props = defineProps<{ class?: HTMLAttributes['class'] }>()
</script>

<template>
	<section
		:class="
			cn(
				'rounded-xl border border-surface-4 bg-surface-2 text-card-foreground shadow-sm',
				props.class,
			)
		"
	>
		<slot />
	</section>
</template>
