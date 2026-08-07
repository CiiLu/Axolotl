import type {
	CustomItem,
	CustomTag,
	JavaVersionId,
	RecipeGeneratorStore,
	RecipeSlot,
	RecipeState,
	SlotValue,
	TagValue,
} from './types.ts'
import {
	ALL_RECIPE_TYPES,
	coerceRecipeTypeForVersion,
	getSupportedRecipeTypes,
	isJavaVersionId,
	LATEST_JAVA_VERSION,
} from './versions.ts'

export const RECIPE_GENERATOR_STORAGE_KEY = 'axolotl.lab.recipe-generator.v1'
export const LEGACY_RECIPE_GENERATOR_STORAGE_KEY = 'axolotl.lab.recipe-generator'

type StorageLike = Pick<Storage, 'getItem' | 'setItem'>

const ALL_SLOTS = new Set<RecipeSlot>([
	'crafting.1',
	'crafting.2',
	'crafting.3',
	'crafting.4',
	'crafting.5',
	'crafting.6',
	'crafting.7',
	'crafting.8',
	'crafting.9',
	'crafting.result',
	'cooking.ingredient',
	'cooking.result',
	'stonecutter.ingredient',
	'stonecutter.result',
	'smithing.template',
	'smithing.base',
	'smithing.addition',
	'smithing.result',
])

export function createDefaultRecipeState(
	type: RecipeState['recipeType'] = 'crafting',
): RecipeState {
	return {
		id: crypto.randomUUID(),
		recipeType: type,
		group: '',
		category: '',
		showNotification: true,
		nameMode: 'auto',
		name: '',
		slots: {},
		crafting: {
			shapeless: false,
			keepWhitespace: false,
			twoByTwo: false,
		},
		cooking: {
			time: null,
			experience: 0,
		},
		smithing: {
			trimPattern: '',
		},
	}
}

export function createDefaultRecipeGeneratorStore(): RecipeGeneratorStore {
	const recipe = createDefaultRecipeState()
	return {
		version: 1,
		selectedVersion: LATEST_JAVA_VERSION,
		recipes: [recipe],
		selectedRecipeId: recipe.id,
		customItems: [],
		customTags: [],
	}
}

export function loadRecipeGeneratorStore(
	storage: StorageLike | null = getBrowserStorage(),
): RecipeGeneratorStore {
	const fallback = createDefaultRecipeGeneratorStore()
	if (!storage) return fallback
	try {
		const raw = storage.getItem(RECIPE_GENERATOR_STORAGE_KEY)
		if (raw) return sanitizeRecipeGeneratorStore(JSON.parse(raw), fallback)
		const legacy = storage.getItem(LEGACY_RECIPE_GENERATOR_STORAGE_KEY)
		return legacy ? sanitizeRecipeGeneratorStore(JSON.parse(legacy), fallback) : fallback
	} catch {
		return fallback
	}
}

export function saveRecipeGeneratorStore(
	store: RecipeGeneratorStore,
	storage: StorageLike | null = getBrowserStorage(),
): void {
	if (!storage) return
	storage.setItem(RECIPE_GENERATOR_STORAGE_KEY, JSON.stringify(sanitizeRecipeGeneratorStore(store)))
}

export function sanitizeRecipeGeneratorStore(
	value: unknown,
	fallback = createDefaultRecipeGeneratorStore(),
): RecipeGeneratorStore {
	if (!value || typeof value !== 'object') return fallback
	const source = value as Partial<RecipeGeneratorStore> & {
		selectedVersion?: unknown
		recipes?: unknown
		selectedRecipeId?: unknown
		customItems?: unknown
		customTags?: unknown
	}
	const recipes = sanitizeRecipes(source.recipes)
	const selectedVersion = isJavaVersionId(source.selectedVersion)
		? source.selectedVersion
		: fallback.selectedVersion
	const selectedRecipeId =
		typeof source.selectedRecipeId === 'string' &&
		recipes.some((recipe) => recipe.id === source.selectedRecipeId)
			? source.selectedRecipeId
			: (recipes[0]?.id ?? fallback.selectedRecipeId)
	return {
		version: 1,
		selectedVersion,
		recipes,
		selectedRecipeId,
		customItems: sanitizeCustomItems(source.customItems),
		customTags: sanitizeCustomTags(source.customTags),
	}
}

export function migrateRecipeGeneratorStore(value: unknown): RecipeGeneratorStore {
	return sanitizeRecipeGeneratorStore(value)
}

function sanitizeRecipes(value: unknown): RecipeState[] {
	if (!Array.isArray(value)) return [createDefaultRecipeState()]
	const recipes = value
		.map((recipe) => sanitizeRecipeState(recipe))
		.filter((recipe): recipe is RecipeState => recipe !== null)
	return recipes.length ? recipes : [createDefaultRecipeState()]
}

function sanitizeRecipeState(value: unknown): RecipeState | null {
	if (!value || typeof value !== 'object') return null
	const source = value as Partial<RecipeState>
	const type = ALL_RECIPE_TYPES.includes(source.recipeType as RecipeState['recipeType'])
		? (source.recipeType as RecipeState['recipeType'])
		: 'crafting'
	const recipe = createDefaultRecipeState(type)
	if (typeof source.id === 'string' && source.id) recipe.id = source.id.slice(0, 128)
	recipe.group = typeof source.group === 'string' ? source.group.slice(0, 256) : ''
	recipe.category = typeof source.category === 'string' ? source.category.slice(0, 64) : ''
	recipe.showNotification = source.showNotification !== false
	recipe.nameMode = source.nameMode === 'manual' ? 'manual' : 'auto'
	recipe.name = typeof source.name === 'string' ? source.name.slice(0, 256) : ''
	recipe.slots = sanitizeSlots(source.slots)
	if (source.crafting && typeof source.crafting === 'object') {
		recipe.crafting.shapeless = source.crafting.shapeless === true
		recipe.crafting.keepWhitespace = source.crafting.keepWhitespace === true
		recipe.crafting.twoByTwo = source.crafting.twoByTwo === true
	}
	if (source.cooking && typeof source.cooking === 'object') {
		recipe.cooking.time =
			typeof source.cooking.time === 'number' && Number.isFinite(source.cooking.time)
				? Math.max(1, Math.round(source.cooking.time))
				: null
		recipe.cooking.experience =
			typeof source.cooking.experience === 'number' && Number.isFinite(source.cooking.experience)
				? Math.max(0, Math.min(100, source.cooking.experience))
				: 0
	}
	if (source.smithing && typeof source.smithing === 'object') {
		recipe.smithing.trimPattern =
			typeof source.smithing.trimPattern === 'string'
				? source.smithing.trimPattern.slice(0, 128)
				: ''
	}
	return recipe
}

function sanitizeSlots(value: unknown): Partial<Record<RecipeSlot, SlotValue>> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
	const slots: Partial<Record<RecipeSlot, SlotValue>> = {}
	for (const [slot, slotValue] of Object.entries(value)) {
		if (!ALL_SLOTS.has(slot as RecipeSlot)) continue
		const sanitized = sanitizeSlotValue(slotValue)
		if (sanitized) slots[slot as RecipeSlot] = sanitized
	}
	return slots
}

function sanitizeSlotValue(value: unknown): SlotValue | null {
	if (!value || typeof value !== 'object') return null
	const source = value as Partial<SlotValue>
	if (source.kind === 'item' && typeof source.id === 'string') {
		return { kind: 'item', id: source.id.slice(0, 512), ...sanitizeCount(source.count) }
	}
	if (source.kind === 'custom_item' && typeof source.uid === 'string') {
		return { kind: 'custom_item', uid: source.uid.slice(0, 128), ...sanitizeCount(source.count) }
	}
	if (source.kind === 'vanilla_tag' && typeof source.id === 'string') {
		return { kind: 'vanilla_tag', id: source.id.slice(0, 512) }
	}
	if (source.kind === 'custom_tag' && typeof source.uid === 'string') {
		return { kind: 'custom_tag', uid: source.uid.slice(0, 128) }
	}
	return null
}

function sanitizeCount(value: unknown): { count?: number } {
	return typeof value === 'number' && Number.isFinite(value) && value > 0
		? { count: Math.min(999, Math.round(value)) }
		: {}
}

function sanitizeCustomItems(value: unknown): CustomItem[] {
	if (!Array.isArray(value)) return []
	return value.flatMap((item) => {
		if (!item || typeof item !== 'object') return []
		const source = item as Partial<CustomItem>
		if (typeof source.uid !== 'string' || typeof source.id !== 'string') return []
		return [
			{
				uid: source.uid.slice(0, 128),
				id: source.id.slice(0, 512),
				name: typeof source.name === 'string' ? source.name.slice(0, 128) : source.id,
				texture: typeof source.texture === 'string' ? source.texture.slice(0, 2_000) : '',
				createdAt:
					typeof source.createdAt === 'string'
						? source.createdAt.slice(0, 64)
						: new Date().toISOString(),
			},
		]
	})
}

function sanitizeCustomTags(value: unknown): CustomTag[] {
	if (!Array.isArray(value)) return []
	return value.flatMap((tag) => {
		if (!tag || typeof tag !== 'object') return []
		const source = tag as Partial<CustomTag>
		if (typeof source.uid !== 'string' || typeof source.id !== 'string') return []
		const values = Array.isArray(source.values)
			? source.values.flatMap((entry) => {
					if (!entry || typeof entry !== 'object') return []
					const tagValue = entry as Partial<TagValue>
					if (typeof tagValue.id !== 'string') return []
					if (tagValue.type !== 'tag' && tagValue.type !== 'item') return []
					return [{ type: tagValue.type, id: tagValue.id.slice(0, 512) }]
				})
			: []
		return [
			{
				uid: source.uid.slice(0, 128),
				id: source.id.slice(0, 512),
				values: values.slice(0, 2_000),
			},
		]
	})
}

function getBrowserStorage(): StorageLike | null {
	return typeof window === 'undefined' ? null : window.localStorage
}

export function ensureRecipeTypeForVersion(
	recipes: RecipeState[],
	version: JavaVersionId,
): RecipeState[] {
	const supported = new Set(getSupportedRecipeTypes(version))
	return recipes.map((recipe) =>
		supported.has(recipe.recipeType)
			? recipe
			: { ...recipe, recipeType: coerceRecipeTypeForVersion(recipe.recipeType, version) },
	)
}
