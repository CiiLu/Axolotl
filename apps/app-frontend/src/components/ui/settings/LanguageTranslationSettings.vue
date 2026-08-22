<script setup lang="ts">
import LanguageSettings from './LanguageSettings.vue'
import TranslationSettings from './TranslationSettings.vue'
</script>

<template>
	<div class="settings-composite-page">
		<LanguageSettings />
		<TranslationSettings />
	</div>
</template>

<style scoped>
.settings-composite-page {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xl);
}
</style>
