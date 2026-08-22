<script setup lang="ts">
import ResourceManagementSettings from './ResourceManagementSettings.vue'
import StorageSettings from './StorageSettings.vue'
</script>

<template>
	<div class="settings-composite-page">
		<ResourceManagementSettings scope="storage-backups" />
		<StorageSettings />
	</div>
</template>

<style scoped>
.settings-composite-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
}
</style>
