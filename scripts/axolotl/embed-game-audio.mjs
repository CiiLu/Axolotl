import fs from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const DEFAULT_GAME_DIR = fileURLToPath(
	new URL('../../apps/app-frontend/public/easteregg/games/', import.meta.url),
)
const MIME = 'audio/mpeg'

async function main() {
	const gameDir = path.resolve(process.argv[2] ?? DEFAULT_GAME_DIR)
	const gameFile = path.join(gameDir, 'game.html')

	if (!existsSync(gameFile)) {
		throw new Error(`game.html not found in ${gameDir}`)
	}

	const entries = await fs.readdir(gameDir, { withFileTypes: true })
	const audioFiles = entries
		.filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith('.mp3'))
		.map((entry) => entry.name)

	if (audioFiles.length === 0) {
		console.log(`No mp3 files found in ${gameDir} — nothing to embed.`)
		return
	}

	let html = await fs.readFile(gameFile, 'utf-8')

	for (const filename of audioFiles) {
		const content = await fs.readFile(path.join(gameDir, filename))
		const dataUri = `data:${MIME};base64,${content.toString('base64')}`
		const stem = filename.slice(0, -4)
		const escapedStem = stem.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
		const reference = new RegExp(`\\./${escapedStem}\\.mp3(?:\\?[^'"\\s)]*)?`, 'g')
		const updated = html.replace(reference, () => dataUri)

		if (updated === html) {
			console.warn(`Warning: no reference to ${filename} found in game.html`)
		} else {
			console.log(`Embedded ${filename} (${(content.length / 1024).toFixed(0)} KiB, ${dataUri.length} chars)`)
		}
		html = updated

		await fs.unlink(path.join(gameDir, filename))
		console.log(`Deleted ${filename}`)
	}

	await fs.writeFile(gameFile, html)
	console.log(`Wrote self-contained ${gameFile} (${(Buffer.byteLength(html) / 1024).toFixed(0)} KiB)`)
}

main().catch((error) => {
	console.error(error)
	process.exitCode = 1
})
