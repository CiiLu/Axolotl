<script setup lang="ts">
import MultiplayerSettings from './MultiplayerSettings.vue'
import ResourceManagementSettings from './ResourceManagementSettings.vue'
</script>

<template>
	<div class="settings-composite-page">
		<ResourceManagementSettings scope="network-multiplayer" />
		<MultiplayerSettings />
	</div>
</template>

<style scoped>
.settings-composite-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
}
</style>
