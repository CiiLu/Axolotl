import type { InstanceSchematicFile } from './backend'

export type InstanceSchematicFolderRow = {
	kind: 'folder'
	path: string
	name: string
	depth: number
	fileCount: number
	expanded: boolean
}

export type InstanceSchematicFileRow = {
	kind: 'file'
	file: InstanceSchematicFile
	depth: number
	parentPath: string
}

export type InstanceSchematicRow = InstanceSchematicFolderRow | InstanceSchematicFileRow

type SchematicFolderNode = {
	path: string
	name: string
	directFiles: InstanceSchematicFile[]
	children: Map<string, SchematicFolderNode>
	parent?: SchematicFolderNode
	fileCount: number
}

function createSchematicFolderNode(path: string, name: string): SchematicFolderNode {
	return {
		path,
		name,
		directFiles: [],
		children: new Map(),
		fileCount: 0,
	}
}

function schematicPathSegments(relativePath: string): string[] {
	return relativePath.split(/[\\/]/).filter(Boolean)
}

function schematicParentPath(relativePath: string): string {
	return schematicPathSegments(relativePath).slice(0, -1).join('/')
}

function buildSchematicFolderTree(files: readonly InstanceSchematicFile[]): SchematicFolderNode {
	const root = createSchematicFolderNode('', '')
	for (const file of files) {
		const segments = schematicPathSegments(file.relativePath)
		let node = root
		let path = ''
		for (let index = 0; index < segments.length - 1; index += 1) {
			path = path ? `${path}/${segments[index]}` : segments[index]
			let child = node.children.get(path)
			if (!child) {
				child = createSchematicFolderNode(path, segments[index])
				child.parent = node
				node.children.set(path, child)
			}
			node = child
		}
		node.directFiles.push(file)
		for (
			let countNode: SchematicFolderNode | undefined = node;
			countNode;
			countNode = countNode.parent
		) {
			countNode.fileCount += 1
		}
	}
	return root
}

function appendFolderRows(
	node: SchematicFolderNode,
	depth: number,
	expandedFolders: ReadonlySet<string>,
	rows: InstanceSchematicRow[],
	locale: string,
) {
	const children = [...node.children.values()].sort((left, right) =>
		left.name.localeCompare(right.name, locale, { sensitivity: 'base' }),
	)
	const files = [...node.directFiles].sort((left, right) =>
		left.fileName.localeCompare(right.fileName, locale, { sensitivity: 'base' }),
	)
	let folderIndex = 0
	let fileIndex = 0
	while (folderIndex < children.length || fileIndex < files.length) {
		const folder = children[folderIndex]
		const file = files[fileIndex]
		if (
			!folder ||
			(file && folder.name.localeCompare(file.fileName, locale, { sensitivity: 'base' }) > 0)
		) {
			rows.push({
				kind: 'file',
				file,
				depth,
				parentPath: node.path,
			})
			fileIndex += 1
			continue
		}
		rows.push({
			kind: 'folder',
			path: folder.path,
			name: folder.name,
			depth,
			fileCount: folder.fileCount,
			expanded: expandedFolders.has(folder.path),
		})
		if (expandedFolders.has(folder.path)) {
			appendFolderRows(folder, depth + 1, expandedFolders, rows, locale)
		}
		folderIndex += 1
	}
}

export function collectSchematicFolders(files: readonly InstanceSchematicFile[]): string[] {
	const folders = new Set<string>()
	for (const file of files) {
		const segments = schematicPathSegments(file.relativePath)
		for (let index = 1; index < segments.length; index += 1) {
			folders.add(segments.slice(0, index).join('/'))
		}
	}
	return [...folders].sort()
}

export function buildInstanceSchematicRows(
	files: readonly InstanceSchematicFile[],
	expandedFolders: ReadonlySet<string>,
	searchQuery: string,
	locale = 'en',
): InstanceSchematicRow[] {
	const query = searchQuery.trim().toLocaleLowerCase(locale)
	if (query) {
		return files
			.filter((file) => file.relativePath.toLocaleLowerCase(locale).includes(query))
			.map((file) => ({
				kind: 'file',
				file,
				depth: 0,
				parentPath: schematicParentPath(file.relativePath),
			}))
	}

	const rows: InstanceSchematicRow[] = []
	appendFolderRows(buildSchematicFolderTree(files), 0, expandedFolders, rows, locale)
	return rows
}
