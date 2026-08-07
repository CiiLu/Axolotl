import { BlocksIcon, BoxIcon, LanguagesIcon, PaletteIcon, WorldIcon } from '@modrinth/assets'
import type { Component } from 'vue'

import GradientTextLab from '@/pages/LabGradientText.vue'
import ModTranslationLab from '@/pages/LabModTranslation.vue'
import RecipeGeneratorLab from '@/pages/LabRecipeGenerator.vue'
import SchematicPreviewLab from '@/pages/LabSchematicPreview.vue'
import SeedMapLab from '@/pages/LabSeedMap.vue'

export type LabToolDefinition = {
	id: string
	category: 'creation' | 'maintenance' | 'world'
	route: string
	icon: Component
	component: Component
	title: string
	description: string
}

export const labTools: readonly LabToolDefinition[] = [
	{
		id: 'gradient-text',
		category: 'creation',
		route: '/lab/gradient-text',
		icon: PaletteIcon,
		component: GradientTextLab,
		title: 'Gradient text generator',
		description: 'Create Minecraft-ready gradient text without a browser.',
	},
	{
		id: 'recipe-generator',
		category: 'creation',
		route: '/lab/recipe-generator',
		icon: BlocksIcon,
		component: RecipeGeneratorLab,
		title: 'Recipe generator',
		description: 'Create Minecraft Java data pack recipes from local item and tag data.',
	},
	{
		id: 'seed-map',
		category: 'world',
		route: '/lab/seed-map',
		icon: WorldIcon,
		component: SeedMapLab,
		title: 'Seed map',
		description: 'Explore a Minecraft seed locally with biomes, structures, and saved markers.',
	},
	{
		id: 'schematic-preview',
		category: 'creation',
		route: '/lab/schematic-preview',
		icon: BoxIcon,
		component: SchematicPreviewLab,
		title: 'Schematic workshop',
		description: 'Quickly preview and edit your schematics.',
	},
	{
		id: 'mod-translation',
		category: 'maintenance',
		route: '/lab/mod-translation',
		icon: LanguagesIcon,
		component: ModTranslationLab,
		title: 'Mod translation',
		description: 'Translate any Minecraft mod JAR into Simplified Chinese.',
	},
]

export function getLabTool(id: string): LabToolDefinition | undefined {
	return labTools.find((tool) => tool.id === id)
}
