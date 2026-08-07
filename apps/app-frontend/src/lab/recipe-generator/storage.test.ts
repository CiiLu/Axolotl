import assert from 'node:assert/strict'
import test from 'node:test'

import {
	createDefaultRecipeGeneratorStore,
	loadRecipeGeneratorStore,
	migrateRecipeGeneratorStore,
	RECIPE_GENERATOR_STORAGE_KEY,
	sanitizeRecipeGeneratorStore,
	saveRecipeGeneratorStore,
} from './storage.ts'

function memoryStorage(initial: Record<string, string> = {}) {
	const values = new Map(Object.entries(initial))
	return {
		getItem: (key: string) => values.get(key) ?? null,
		setItem: (key: string, value: string) => values.set(key, value),
	}
}

test('creates a default store with one crafting recipe', () => {
	const store = createDefaultRecipeGeneratorStore()
	assert.equal(store.version, 1)
	assert.equal(store.selectedVersion, '26.2')
	assert.equal(store.recipes.length, 1)
	assert.equal(store.recipes[0].recipeType, 'crafting')
})

test('sanitizes malformed persisted data', () => {
	const store = sanitizeRecipeGeneratorStore({
		version: 1,
		selectedVersion: '999',
		recipes: [{ id: 'r1', recipeType: 'bogus', slots: { 'crafting.1': { kind: 'nope' } } }],
		selectedRecipeId: 'missing',
		customItems: [{ uid: 'u1', id: 12 }],
		customTags: 'bad',
	})
	assert.equal(store.selectedVersion, '26.2')
	assert.equal(store.recipes.length, 1)
	assert.equal(store.recipes[0].recipeType, 'crafting')
	assert.deepEqual(store.recipes[0].slots, {})
	assert.deepEqual(store.customItems, [])
	assert.deepEqual(store.customTags, [])
	assert.equal(store.selectedRecipeId, store.recipes[0].id)
})

test('migrates unversioned stores', () => {
	const migrated = migrateRecipeGeneratorStore({
		recipes: [
			{
				id: 'legacy',
				recipeType: 'smelting',
				slots: { 'cooking.ingredient': { kind: 'item', id: 'minecraft:beef' } },
			},
		],
	})
	assert.equal(migrated.version, 1)
	assert.equal(migrated.recipes[0].recipeType, 'smelting')
	assert.equal(migrated.selectedRecipeId, 'legacy')
})

test('round-trips through localStorage-compatible storage', () => {
	const storage = memoryStorage()
	const store = createDefaultRecipeGeneratorStore()
	store.recipes[0].slots['crafting.1'] = { kind: 'item', id: 'minecraft:oak_planks' }
	saveRecipeGeneratorStore(store, storage)
	const loaded = loadRecipeGeneratorStore(storage)
	assert.equal(loaded.recipes[0].slots['crafting.1']?.kind, 'item')
	assert.equal(storage.getItem(RECIPE_GENERATOR_STORAGE_KEY) !== null, true)
})
