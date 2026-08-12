import { defineMessage, type MessageDescriptor } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'

import { isBuiltInInstanceIcon } from './instance-icon-frame'

export interface BuiltInInstanceIcon {
	id: string
	name: MessageDescriptor
	url: string
}

export const builtInInstanceIcons: BuiltInInstanceIcon[] = [
	{
		id: 'bread',
		name: defineMessage({ id: 'app.instance.icon-picker.icon.bread', defaultMessage: 'Bread' }),
		url: new URL('../assets/instance-icons/bread.png', import.meta.url).href,
	},
	{
		id: 'carrot',
		name: defineMessage({ id: 'app.instance.icon-picker.icon.carrot', defaultMessage: 'Carrot' }),
		url: new URL('../assets/instance-icons/carrot.png', import.meta.url).href,
	},
	{
		id: 'cooked-chicken',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.cooked-chicken',
			defaultMessage: 'Cooked Chicken',
		}),
		url: new URL('../assets/instance-icons/cooked-chicken.png', import.meta.url).href,
	},
	{
		id: 'crafting-table',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.crafting-table',
			defaultMessage: 'Crafting Table',
		}),
		url: new URL('../assets/instance-icons/crafting-table.png', import.meta.url).href,
	},
	{
		id: 'diamond-axe',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.diamond-axe',
			defaultMessage: 'Diamond Axe',
		}),
		url: new URL('../assets/instance-icons/diamond-axe.png', import.meta.url).href,
	},
	{
		id: 'diamond-block',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.diamond-block',
			defaultMessage: 'Diamond Block',
		}),
		url: new URL('../assets/instance-icons/diamond-block.png', import.meta.url).href,
	},
	{
		id: 'diamond-sword',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.diamond-sword',
			defaultMessage: 'Diamond Sword',
		}),
		url: new URL('../assets/instance-icons/diamond-sword.png', import.meta.url).href,
	},
	{
		id: 'end-stone',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.end-stone',
			defaultMessage: 'End Stone',
		}),
		url: new URL('../assets/instance-icons/end-stone.png', import.meta.url).href,
	},
	{
		id: 'furnace',
		name: defineMessage({ id: 'app.instance.icon-picker.icon.furnace', defaultMessage: 'Furnace' }),
		url: new URL('../assets/instance-icons/furnace.png', import.meta.url).href,
	},
	{
		id: 'glass-bottle',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.glass-bottle',
			defaultMessage: 'Glass Bottle',
		}),
		url: new URL('../assets/instance-icons/glass-bottle.png', import.meta.url).href,
	},
	{
		id: 'golden-apple',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.golden-apple',
			defaultMessage: 'Golden Apple',
		}),
		url: new URL('../assets/instance-icons/golden-apple.png', import.meta.url).href,
	},
	{
		id: 'gold-block',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.gold-block',
			defaultMessage: 'Gold Block',
		}),
		url: new URL('../assets/instance-icons/gold-block.png', import.meta.url).href,
	},
	{
		id: 'grass-block',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.grass-block',
			defaultMessage: 'Grass Block',
		}),
		url: new URL('../assets/instance-icons/grass-block.png', import.meta.url).href,
	},
	{
		id: 'iron-block',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.iron-block',
			defaultMessage: 'Iron Block',
		}),
		url: new URL('../assets/instance-icons/iron-block.png', import.meta.url).href,
	},
	{
		id: 'item-frame',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.item-frame',
			defaultMessage: 'Item Frame',
		}),
		url: new URL('../assets/instance-icons/item-frame.png', import.meta.url).href,
	},
	{
		id: 'netherrack',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.netherrack',
			defaultMessage: 'Netherrack',
		}),
		url: new URL('../assets/instance-icons/netherrack.png', import.meta.url).href,
	},
	{
		id: 'oak-sapling',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.oak-sapling',
			defaultMessage: 'Oak Sapling',
		}),
		url: new URL('../assets/instance-icons/oak-sapling.png', import.meta.url).href,
	},
	{
		id: 'stone',
		name: defineMessage({ id: 'app.instance.icon-picker.icon.stone', defaultMessage: 'Stone' }),
		url: new URL('../assets/instance-icons/stone.png', import.meta.url).href,
	},
	{
		id: 'totem-of-undying',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.totem-of-undying',
			defaultMessage: 'Totem of Undying',
		}),
		url: new URL('../assets/instance-icons/totem-of-undying.png', import.meta.url).href,
	},
	{
		id: 'water-bucket',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.water-bucket',
			defaultMessage: 'Water Bucket',
		}),
		url: new URL('../assets/instance-icons/water-bucket.png', import.meta.url).href,
	},
	{
		id: 'anvil',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.anvil',
			defaultMessage: 'Anvil',
		}),
		url: new URL('../assets/instance-icons/anvil.png', import.meta.url).href,
	},
	{
		id: 'fabric',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.fabric',
			defaultMessage: 'Fabric',
		}),
		url: new URL('../assets/instance-icons/Fabric.png', import.meta.url).href,
	},
	{
		id: 'neoforge',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.neoforge',
			defaultMessage: 'NeoForge',
		}),
		url: new URL('../assets/instance-icons/NeoForge.png', import.meta.url).href,
	},
	{
		id: 'quilt',
		name: defineMessage({
			id: 'app.instance.icon-picker.icon.quilt',
			defaultMessage: 'Quilt',
		}),
		url: new URL('../assets/instance-icons/Quilt.png', import.meta.url).href,
	},
]

const builtInInstanceIconMap = new Map(builtInInstanceIcons.map((icon) => [icon.id, icon]))

const loaderIconIds: Record<string, string> = {
	vanilla: 'grass-block',
	fabric: 'fabric',
	forge: 'anvil',
	neoforge: 'neoforge',
	quilt: 'quilt',
}

export interface DisplayInstanceIcon {
	url: string | null
	frameless: boolean
}

export function getLoaderInstanceIcon(
	loader: string | null | undefined,
): BuiltInInstanceIcon | undefined {
	if (!loader) return undefined
	const iconId = loaderIconIds[loader]
	return (
		(iconId ? builtInInstanceIconMap.get(iconId) : undefined) ?? builtInInstanceIconMap.get('stone')
	)
}

export function getDisplayInstanceIcon(
	iconPath: string | null | undefined,
	loader: string | null | undefined,
): DisplayInstanceIcon {
	const fallbackIcon = getLoaderInstanceIcon(loader)
	return {
		url: iconPath ? convertFileSrc(iconPath) : (fallbackIcon?.url ?? null),
		frameless: isBuiltInInstanceIcon(iconPath) || !!fallbackIcon,
	}
}
