<template>
	<div />
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { upgradeResultDownloadsLocation } from './result'

const route = useRoute()
const router = useRouter()

onMounted(async () => {
	const job = typeof route.query.job === 'string' ? route.query.job : null
	await router.replace(job ? upgradeResultDownloadsLocation(job) : { path: '/downloads' })
})
</script>
