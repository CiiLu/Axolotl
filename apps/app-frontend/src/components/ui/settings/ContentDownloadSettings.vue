<script setup lang="ts">
import AppearanceSettings from './AppearanceSettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
</script>

<template>
	<div class="settings-composite-page">
		<AppearanceSettings scope="content-downloads" />
		<ResourceManagementSettings scope="content-downloads" />
	</div>
</template>

<style scoped>
.settings-composite-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
}
</style>
