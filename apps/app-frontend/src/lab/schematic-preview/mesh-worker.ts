import type {
	BlockDefinition,
	BlockModel,
	Cull,
	Identifier,
	type TextureAtlasProvider,
} from 'deepslate'

import type { SchematicBlockState } from './backend'
import { isSchematicAir } from './editing'
import {
	applySeamlessSchematicGlassUvs,
	getSchematicSpecialBlockMesh,
	isSchematicTranslucent,
	SCHEMATIC_DIRECTIONS,
	schematicBlockAt,
	type SchematicDirection,
	type SchematicNeighborFaces,
	shouldCullSchematicFace,
} from './meshing'
import type { SchematicWorkerResources } from './resources'

type WorkerInitMessage = {
	type: 'init'
	epoch: number
	palette: SchematicBlockState[]
	resources: SchematicWorkerResources
	seamlessGlass: boolean
}

type WorkerMeshMessage = {
	type: 'mesh'
	epoch: number
	jobId: string
	regionId: string
	chunkPosition: [number, number, number]
	blocks: ArrayBuffer
	neighborFaces?: Partial<Record<SchematicDirection, ArrayBuffer>>
}

export type SchematicMeshWorkerRequest = WorkerInitMessage | WorkerMeshMessage

export type SchematicMeshData = {
	positions: Float32Array
	normals: Float32Array
	uvs: Float32Array
	colors: Float32Array
	blockPositions: Float32Array
}

export type SchematicMeshWorkerResponse =
	| { type: 'ready'; epoch: number; warnings: string[] }
	| {
			type: 'mesh'
			epoch: number
			jobId: string
			regionId: string
			chunkPosition: [number, number, number]
			opaque: SchematicMeshData
			translucent: SchematicMeshData
			missing: string[]
	  }
	| { type: 'error'; epoch: number; jobId?: string; message: string }

type MeshBuffers = {
	positions: number[]
	normals: number[]
	uvs: number[]
	colors: number[]
	blockPositions: number[]
}

let activeEpoch = 0
let palette: SchematicBlockState[] = []
let blockDefinitions: Record<string, BlockDefinition> = {}
let blockModels: Record<string, BlockModel> = {}
let defaultBlockProperties: Record<string, Record<string, string>> = {}
let textureUvs: Record<string, [number, number, number, number]> = {}
let missingTextureUv: [number, number, number, number] = [0, 0, 1, 1]
let seamlessGlass = true

const workerScope = self as DedicatedWorkerGlobalScope

function emptyBuffers(): MeshBuffers {
	return { positions: [], normals: [], uvs: [], colors: [], blockPositions: [] }
}

function toMeshData(buffers: MeshBuffers): SchematicMeshData {
	return {
		positions: new Float32Array(buffers.positions),
		normals: new Float32Array(buffers.normals),
		uvs: new Float32Array(buffers.uvs),
		colors: new Float32Array(buffers.colors),
		blockPositions: new Float32Array(buffers.blockPositions),
	}
}

function transferables(data: SchematicMeshData): Transferable[] {
	return [
		data.positions.buffer,
		data.normals.buffer,
		data.uvs.buffer,
		data.colors.buffer,
		data.blockPositions.buffer,
	]
}

function appendQuad(
	buffers: MeshBuffers,
	vertices: Array<{
		pos: { x: number; y: number; z: number }
		texture?: [number, number]
		color: [number, number, number]
	}>,
	blockPosition: [number, number, number],
) {
	const edge1 = {
		x: vertices[1].pos.x - vertices[0].pos.x,
		y: vertices[1].pos.y - vertices[0].pos.y,
		z: vertices[1].pos.z - vertices[0].pos.z,
	}
	const edge2 = {
		x: vertices[2].pos.x - vertices[0].pos.x,
		y: vertices[2].pos.y - vertices[0].pos.y,
		z: vertices[2].pos.z - vertices[0].pos.z,
	}
	const normal = {
		x: edge1.y * edge2.z - edge1.z * edge2.y,
		y: edge1.z * edge2.x - edge1.x * edge2.z,
		z: edge1.x * edge2.y - edge1.y * edge2.x,
	}
	const length = Math.hypot(normal.x, normal.y, normal.z) || 1
	const order = [0, 1, 2, 0, 2, 3]
	for (const index of order) {
		const vertex = vertices[index]
		buffers.positions.push(vertex.pos.x, vertex.pos.y, vertex.pos.z)
		buffers.normals.push(normal.x / length, normal.y / length, normal.z / length)
		buffers.uvs.push(vertex.texture?.[0] ?? 0, vertex.texture?.[1] ?? 0)
		buffers.colors.push(vertex.color[0], vertex.color[1], vertex.color[2])
		buffers.blockPositions.push(...blockPosition)
	}
}

function appendFallbackCube(
	buffers: MeshBuffers,
	position: [number, number, number],
	cull: ReturnType<typeof Cull.none>,
	blockName: string,
) {
	const [x, y, z] = position
	const [u0, v0, u1, v1] = missingTextureUv
	let hash = 0
	for (const character of blockName) hash = (hash * 31 + character.charCodeAt(0)) | 0
	const color: [number, number, number] = [
		0.55 + ((hash >>> 0) & 0xff) / 640,
		0.55 + ((hash >>> 8) & 0xff) / 640,
		0.55 + ((hash >>> 16) & 0xff) / 640,
	]
	const vertex = (px: number, py: number, pz: number, u: number, v: number) => ({
		pos: { x: px, y: py, z: pz },
		texture: [u, v] as [number, number],
		color,
	})
	const faces: Array<[keyof typeof cull, ReturnType<typeof vertex>[]]> = [
		[
			'up',
			[
				vertex(x, y + 1, z + 1, u0, v1),
				vertex(x + 1, y + 1, z + 1, u1, v1),
				vertex(x + 1, y + 1, z, u1, v0),
				vertex(x, y + 1, z, u0, v0),
			],
		],
		[
			'down',
			[
				vertex(x, y, z, u0, v0),
				vertex(x + 1, y, z, u1, v0),
				vertex(x + 1, y, z + 1, u1, v1),
				vertex(x, y, z + 1, u0, v1),
			],
		],
		[
			'south',
			[
				vertex(x, y, z + 1, u0, v1),
				vertex(x + 1, y, z + 1, u1, v1),
				vertex(x + 1, y + 1, z + 1, u1, v0),
				vertex(x, y + 1, z + 1, u0, v0),
			],
		],
		[
			'north',
			[
				vertex(x + 1, y, z, u0, v1),
				vertex(x, y, z, u1, v1),
				vertex(x, y + 1, z, u1, v0),
				vertex(x + 1, y + 1, z, u0, v0),
			],
		],
		[
			'east',
			[
				vertex(x + 1, y, z + 1, u0, v1),
				vertex(x + 1, y, z, u1, v1),
				vertex(x + 1, y + 1, z, u1, v0),
				vertex(x + 1, y + 1, z + 1, u0, v0),
			],
		],
		[
			'west',
			[
				vertex(x, y, z, u0, v1),
				vertex(x, y, z + 1, u1, v1),
				vertex(x, y + 1, z + 1, u1, v0),
				vertex(x, y + 1, z, u0, v0),
			],
		],
	]
	for (const [direction, vertices] of faces) {
		if (!cull[direction]) appendQuad(buffers, vertices, position)
	}
}

function initialize(message: WorkerInitMessage) {
	activeEpoch = message.epoch
	palette = message.palette
	defaultBlockProperties = message.resources.defaultBlockProperties
	textureUvs = message.resources.textureUvs
	missingTextureUv = message.resources.missingTextureUv
	seamlessGlass = message.seamlessGlass
	const warnings: string[] = []
	blockDefinitions = {}
	for (const [id, value] of Object.entries(message.resources.blockDefinitions)) {
		try {
			blockDefinitions[id] = BlockDefinition.fromJson(value)
		} catch {
			warnings.push(`Skipped invalid blockstate ${id}`)
		}
	}
	blockModels = {}
	for (const [id, value] of Object.entries(message.resources.blockModels)) {
		try {
			blockModels[id] = BlockModel.fromJson(value)
		} catch {
			warnings.push(`Skipped invalid block model ${id}`)
		}
	}
	const provider = {
		getBlockModel(id: Identifier) {
			return blockModels[id.toString()] ?? null
		},
	}
	for (const [id, model] of Object.entries(blockModels)) {
		try {
			model.flatten(provider)
		} catch {
			Reflect.deleteProperty(blockModels, id)
			warnings.push(`Skipped unresolved block model ${id}`)
		}
	}
	workerScope.postMessage({
		type: 'ready',
		epoch: activeEpoch,
		warnings,
	} satisfies SchematicMeshWorkerResponse)
}

function buildChunk(message: WorkerMeshMessage) {
	if (message.epoch !== activeEpoch) return
	const blocks = new Uint32Array(message.blocks)
	const neighborFaces: SchematicNeighborFaces = {}
	for (const direction of SCHEMATIC_DIRECTIONS) {
		const face = message.neighborFaces?.[direction]
		if (face) neighborFaces[direction] = new Uint32Array(face)
	}
	const opaque = emptyBuffers()
	const translucent = emptyBuffers()
	const missing = new Set<string>()
	const atlas: TextureAtlasProvider = {
		getTextureAtlas: () => ({}) as ImageData,
		getTextureUV: (id) => textureUvs[id.toString()] ?? missingTextureUv,
	}
	const modelProvider = {
		getBlockModel(id: Identifier) {
			return blockModels[id.toString()] ?? null
		},
	}
	const chunkOrigin = message.chunkPosition.map((value) => value * 16) as [number, number, number]

	for (let y = 0; y < 16; y += 1) {
		for (let z = 0; z < 16; z += 1) {
			for (let x = 0; x < 16; x += 1) {
				const paletteIndex = schematicBlockAt(blocks, neighborFaces, x, y, z)
				const state = palette[paletteIndex]
				if (!state || isSchematicAir(state.name)) continue
				const position: [number, number, number] = [
					chunkOrigin[0] + x,
					chunkOrigin[1] + y,
					chunkOrigin[2] + z,
				]
				const cull = {
					west: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x - 1, y, z),
						palette,
						seamlessGlass,
					),
					east: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x + 1, y, z),
						palette,
						seamlessGlass,
					),
					down: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x, y - 1, z),
						palette,
						seamlessGlass,
					),
					up: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x, y + 1, z),
						palette,
						seamlessGlass,
					),
					north: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x, y, z - 1),
						palette,
						seamlessGlass,
					),
					south: shouldCullSchematicFace(
						paletteIndex,
						schematicBlockAt(blocks, neighborFaces, x, y, z + 1),
						palette,
						seamlessGlass,
					),
				}
				const target = isSchematicTranslucent(state.name) ? translucent : opaque
				try {
					const properties = {
						...(defaultBlockProperties[state.name] ?? {}),
						...state.properties,
					}
					const resolvedState = { ...state, properties }
					const mesh = getSchematicSpecialBlockMesh(resolvedState, atlas, cull)
					const definition = blockDefinitions[state.name]
					if (definition) {
						mesh.merge(
							definition.getMesh(
								Identifier.parse(state.name),
								properties,
								atlas,
								modelProvider,
								cull,
							),
						)
					}
					if (mesh.quads.length === 0) {
						missing.add(state.name)
						appendFallbackCube(target, position, cull, state.name)
						continue
					}
					for (const quad of mesh.quads) {
						const vertices = quad.vertices().map((item) => ({
							pos: {
								x: item.pos.x + position[0],
								y: item.pos.y + position[1],
								z: item.pos.z + position[2],
							},
							texture: item.texture ? ([...item.texture] as [number, number]) : undefined,
							color: item.color,
						}))
						applySeamlessSchematicGlassUvs(
							vertices,
							[x, y, z],
							paletteIndex,
							blocks,
							neighborFaces,
							palette,
							seamlessGlass,
						)
						appendQuad(target, vertices, position)
					}
				} catch {
					missing.add(state.name)
					appendFallbackCube(target, position, cull, state.name)
				}
			}
		}
	}

	const response: SchematicMeshWorkerResponse = {
		type: 'mesh',
		epoch: message.epoch,
		jobId: message.jobId,
		regionId: message.regionId,
		chunkPosition: message.chunkPosition,
		opaque: toMeshData(opaque),
		translucent: toMeshData(translucent),
		missing: [...missing],
	}
	if (response.type !== 'mesh') return
	workerScope.postMessage(response, [
		...transferables(response.opaque),
		...transferables(response.translucent),
	])
}

workerScope.onmessage = (event: MessageEvent<SchematicMeshWorkerRequest>) => {
	try {
		if (event.data.type === 'init') initialize(event.data)
		else buildChunk(event.data)
	} catch (error) {
		workerScope.postMessage({
			type: 'error',
			epoch: event.data.epoch,
			jobId: event.data.type === 'mesh' ? event.data.jobId : undefined,
			message: error instanceof Error ? error.message : String(error),
		} satisfies SchematicMeshWorkerResponse)
	}
}
