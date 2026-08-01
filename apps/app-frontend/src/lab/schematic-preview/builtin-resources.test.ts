import assert from 'node:assert/strict'
import { after, before, test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { BlockDefinition, BlockModel, Cull, Identifier } from 'deepslate'
import { createServer, type ViteDevServer } from 'vite'

import type { SchematicBlockState } from './backend.ts'

type BuiltinResourcesModule = typeof import('./builtin-resources.ts')

let server: ViteDevServer
let builtin: BuiltinResourcesModule

before(async () => {
	const appRoot = fileURLToPath(new URL('../../../', import.meta.url))
	server = await createServer({
		root: appRoot,
		configFile: `${appRoot}/vite.config.ts`,
		server: { middlewareMode: true, hmr: false },
		appType: 'custom',
	})
	builtin = (await server.ssrLoadModule(
		'/src/lab/schematic-preview/builtin-resources.ts',
	)) as BuiltinResourcesModule
})

after(async () => {
	await server.close()
})

test('built-in assets render stained glass and sugar cane across supported versions', () => {
	const palette: SchematicBlockState[] = [
		{ name: 'minecraft:air', properties: {} },
		{ name: 'minecraft:white_stained_glass', properties: {} },
		{ name: 'minecraft:sugar_cane', properties: {} },
	]
	for (const version of ['1.21', '26.2']) {
		const resources = builtin.loadBuiltinBlockResources(version, palette)
		assert.equal(resources.defaultBlockProperties['minecraft:sugar_cane']?.age, '0')
		const models = Object.fromEntries(
			Object.entries(resources.blockModels).map(([id, model]) => [id, BlockModel.fromJson(model)]),
		)
		const modelProvider = {
			getBlockModel: (id: Identifier) => models[id.toString()] ?? null,
		}
		for (const model of Object.values(models)) model.flatten(modelProvider)

		const textureUvs = builtin.builtinTextureUvs()
		for (const blockName of ['white_stained_glass', 'sugar_cane']) {
			const state = palette.find((entry) => entry.name === `minecraft:${blockName}`)
			assert.ok(state)
			assert.ok(textureUvs[`minecraft:block/${blockName}`])
			const definition = BlockDefinition.fromJson(resources.blockDefinitions[state.name])
			const mesh = definition.getMesh(
				Identifier.parse(state.name),
				state.properties,
				{
					getTextureAtlas: () => ({}) as ImageData,
					getTextureUV: (id) => textureUvs[id.toString()] ?? [0, 0, 0, 0],
				},
				modelProvider,
				Cull.none(),
			)
			assert.ok(mesh.quads.length > 0, `${version} generated no mesh for ${state.name}`)
		}
	}
})

test('built-in assets expose the complete renderer resource set', () => {
	const textureUvs = builtin.builtinTextureUvs()
	assert.ok(Object.keys(textureUvs).length >= 2600)
	assert.ok(textureUvs['minecraft:block/white_stained_glass'])
	assert.ok(textureUvs['minecraft:block/sugar_cane'])
	assert.deepEqual(builtin.builtinTextureRegion('minecraft:block/white_stained_glass'), {
		x: 80,
		y: 1952,
		width: 16,
		height: 16,
	})
})

test('all built-in block resources remain compatible with Deepslate', () => {
	const palette = builtin.listBuiltinBlockStates()
	const resources = builtin.loadBuiltinBlockResources('26.3-snapshot-6', palette)
	const definitions = Object.fromEntries(
		Object.entries(resources.blockDefinitions).map(([id, definition]) => [
			id,
			BlockDefinition.fromJson(definition),
		]),
	)
	const models = Object.fromEntries(
		Object.entries(resources.blockModels).map(([id, model]) => [id, BlockModel.fromJson(model)]),
	)
	const modelProvider = {
		getBlockModel: (id: Identifier) => models[id.toString()] ?? null,
	}
	for (const model of Object.values(models)) model.flatten(modelProvider)

	const textureUvs = builtin.builtinTextureUvs()
	const atlasProvider = {
		getTextureAtlas: () => ({}) as ImageData,
		getTextureUV: (id: Identifier) => textureUvs[id.toString()] ?? [0, 0, 0, 0],
	}
	for (const state of palette) {
		try {
			definitions[state.name]?.getMesh(
				Identifier.parse(state.name),
				state.properties,
				atlasProvider,
				modelProvider,
				Cull.none(),
			)
		} catch (error) {
			throw new Error(`Unable to render built-in block ${state.name}`, { cause: error })
		}
	}
})

test('external models inherit missing vanilla parents from built-in assets', () => {
	const blockDefinitions: Record<string, unknown> = {
		'example:machine': { variants: { '': { model: 'example:block/machine' } } },
	}
	const blockModels: Record<string, unknown> = {
		'example:block/machine': {
			parent: 'minecraft:block/cube_all',
			textures: { all: 'minecraft:block/white_stained_glass' },
		},
	}
	builtin.completeBuiltinModelDependencies('26.2', blockDefinitions, blockModels)
	assert.ok(blockModels['minecraft:block/cube_all'])
})

test('unqualified model parents use the vanilla namespace', () => {
	const blockDefinitions: Record<string, unknown> = {
		'example:machine': { variants: { '': { model: 'example:block/machine' } } },
	}
	const blockModels: Record<string, unknown> = {
		'example:block/machine': { parent: 'block/cube_all' },
	}
	builtin.completeBuiltinModelDependencies('26.2', blockDefinitions, blockModels)
	assert.ok(blockModels['minecraft:block/cube_all'])
})
