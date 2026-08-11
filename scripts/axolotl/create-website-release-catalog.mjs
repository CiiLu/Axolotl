import fs from 'node:fs/promises'
import { createRequire } from 'node:module'

// 从 app 前端的公告 catalog（apps/app-frontend/src/announcements/catalog.ts，
// 发布时人工维护的唯一数据源）导出网站 changelog 数据。
// 该文件提交到仓库 main 分支后由 CNB 镜像同步，网站 changelog 页面
// 每次用户访问时从 CNB 拉取，国内用户无需访问 GitHub API。
const [outputPath] = process.argv.slice(2)

if (!outputPath) {
	throw new Error('Usage: node create-website-release-catalog.mjs <output.json>')
}

const require = createRequire(new URL('../../apps/app-frontend/package.json', import.meta.url))
const ts = require('typescript')

const catalogSource = await fs.readFile(
	'apps/app-frontend/src/announcements/catalog.ts',
	'utf8',
)
const catalogModule = await import(
	`data:text/javascript;base64,${Buffer.from(
		ts.transpileModule(catalogSource, {
			compilerOptions: {
				module: ts.ModuleKind.ESNext,
				target: ts.ScriptTarget.ES2022,
			},
		}).outputText,
	).toString('base64')}`
)

const announcements = catalogModule.launcherAnnouncements
if (!Array.isArray(announcements) || announcements.length === 0) {
	throw new Error('Announcement catalog does not contain any announcements')
}

const catalog = {
	updated_at: new Date().toISOString(),
	announcements: announcements.map(({ id, version, publishedAt, title, changes, notes, externalUrl }) => ({
		id,
		version,
		publishedAt,
		title,
		changes,
		notes,
		externalUrl,
	})),
}

await fs.writeFile(outputPath, `${JSON.stringify(catalog, null, 2)}\n`)
console.log(`Wrote ${outputPath} with ${catalog.announcements.length} announcements`)
