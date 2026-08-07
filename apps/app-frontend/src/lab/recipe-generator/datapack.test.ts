import assert from 'node:assert/strict'
import test from 'node:test'

import { unzipSync } from 'fflate'

import { createDatapackBlob, createDatapackFiles, createPackMcmeta } from './datapack.ts'

test('creates pack.mcmeta with legacy and ranged pack formats', () => {
	assert.equal(JSON.parse(createPackMcmeta(48)).pack.pack_format, 48)
	assert.deepEqual(JSON.parse(createPackMcmeta([107, 1])).pack, {
		description: 'Generated with Axolotl Recipe Generator',
		min_format: [107, 1],
		max_format: [107, 1],
	})
})

test('builds datapack files with versioned recipe and tag directories', async () => {
	const files = createDatapackFiles(
		'1.21',
		[{ name: 'iron_bars', json: { type: 'minecraft:crafting_shaped' } }],
		[
			{
				namespace: 'crafting',
				id: 'my_tag',
				values: ['minecraft:oak_planks', '#minecraft:planks'],
			},
		],
	)
	assert.deepEqual(
		files.map((file) => file.path),
		['pack.mcmeta', 'data/crafting/recipe/iron_bars.json', 'data/crafting/tags/item/my_tag.json'],
	)
	const blob = createDatapackBlob(files)
	const archive = unzipSync(new Uint8Array(await blob.arrayBuffer()))
	assert.deepEqual(
		JSON.parse(new TextDecoder().decode(archive['data/crafting/recipe/iron_bars.json'])),
		{
			type: 'minecraft:crafting_shaped',
		},
	)
	assert.deepEqual(
		JSON.parse(new TextDecoder().decode(archive['data/crafting/tags/item/my_tag.json'])),
		{ replace: false, values: ['minecraft:oak_planks', '#minecraft:planks'] },
	)
})

test('uses recipes directory before 1.21', () => {
	const files = createDatapackFiles(
		'1.20',
		[{ name: 'iron_bars', json: { type: 'minecraft:crafting_shaped' } }],
		[],
	)
	assert.ok(files.some((file) => file.path === 'data/crafting/recipes/iron_bars.json'))
})

test('rejects datapack export for 1.12', () => {
	assert.throws(() => createDatapackFiles('1.12', [], []))
})
