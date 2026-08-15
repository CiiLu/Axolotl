<script setup lang="ts">
defineProps<{ title: string; description: string }>()
</script>

<template>
	<header class="mb-8 flex flex-wrap items-end justify-between gap-4">
		<div class="min-w-0">
			<h1 class="text-2xl font-semibold tracking-tight sm:text-3xl">{{ title }}</h1>
			<p class="mt-1.5 max-w-3xl text-sm leading-6 text-muted-foreground">{{ description }}</p>
		</div>
		<slot />
	</header>
</template>
