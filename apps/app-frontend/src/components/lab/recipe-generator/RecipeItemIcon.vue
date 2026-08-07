<script setup lang="ts">
import { computed } from 'vue'

import type { SlotDisplay } from '@/lab/recipe-generator/display'
import { TEXTURE_ATLAS_SIZE, type TextureAtlas } from '@/lab/recipe-generator/resources'

const props = withDefaults(
	defineProps<{
		display: SlotDisplay | null
		atlas: TextureAtlas
		size?: number
		showCount?: boolean
	}>(),
	{
		size: 32,
		showCount: true,
	},
)

const region = computed(() => {
	const texture = props.display?.texture
	return texture ? props.atlas.layout[texture] : undefined
})

const contentSize = computed(() => Math.max(0, props.size - 2))

const atlasStyle = computed(() => {
	const currentRegion = region.value
	if (!currentRegion) return undefined
	const [x, y, width, height] = currentRegion
	const scaleX = width > 0 ? contentSize.value / width : 0
	const scaleY = height > 0 ? contentSize.value / height : 0
	const offsetX = Math.round(-x * scaleX)
	const offsetY = Math.round(-y * scaleY)
	return {
		width: `${TEXTURE_ATLAS_SIZE.width * scaleX}px`,
		height: `${TEXTURE_ATLAS_SIZE.height * scaleY}px`,
		maxWidth: 'none',
		maxHeight: 'none',
		transform: `translate(${offsetX}px, ${offsetY}px)`,
	}
})
</script>

<template>
	<div
		class="recipe-item-icon"
		:style="{ width: `${size}px`, height: `${size}px` }"
		:title="display?.label"
	>
		<div v-if="region" class="recipe-item-atlas">
			<img :src="atlas.url" alt="" :style="atlasStyle" />
		</div>
		<img
			v-else-if="display?.texture"
			:src="display.texture"
			alt=""
			class="recipe-item-custom"
			:style="{ width: '100%', height: '100%' }"
		/>
		<span v-else class="recipe-item-empty" aria-hidden="true"></span>
		<span v-if="showCount && display?.count && display.count > 1" class="recipe-item-count">{{
			display.count
		}}</span>
	</div>
</template>

<style scoped>
.recipe-item-icon {
	position: relative;
	display: inline-block;
	flex: 0 0 auto;
	overflow: hidden;
	border: 1px solid var(--surface-5);
	box-sizing: border-box;
}

.recipe-item-atlas {
	width: 100%;
	height: 100%;
	overflow: hidden;
}

.recipe-item-atlas img {
	display: block;
	image-rendering: pixelated;
}

.recipe-item-custom {
	display: block;
	image-rendering: pixelated;
	object-fit: contain;
}

.recipe-item-empty {
	display: block;
	width: 100%;
	height: 100%;
	background: repeating-conic-gradient(var(--surface-5) 0% 25%, var(--surface-3) 0% 50%);
	background-size: 8px 8px;
}

.recipe-item-count {
	position: absolute;
	right: 1px;
	bottom: 0;
	color: #fff;
	font-size: 10px;
	font-weight: 700;
	line-height: 1;
	text-shadow: 1px 1px 0 #000;
	pointer-events: none;
}
</style>
