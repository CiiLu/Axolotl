import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

const [assetsDir, tag, outputPath] = process.argv.slice(2)

if (!assetsDir || !tag || !outputPath) {
	throw new Error(
		'Usage: node create-update-server-catalog.mjs <assets-dir> <version-tag> <output.json>',
	)
}

const version = tag.replace(/^v/, '')
const ignoredFiles = new Set(['latest.json', 'release.json', path.basename(outputPath)])
const files = fs
	.readdirSync(assetsDir, { withFileTypes: true })
	.filter((entry) => entry.isFile())
	.map((entry) => entry.name)
	.filter((filename) => !ignoredFiles.has(filename))
	.sort()

function updaterTargets(filename) {
	if (filename.endsWith('_universal.app.tar.gz')) return ['darwin-aarch64', 'darwin-x86_64']
	if (filename.endsWith('_aarch64.AppImage.tar.gz')) return ['linux-aarch64']
	if (filename.endsWith('_amd64.AppImage.tar.gz')) return ['linux-x86_64']
	if (filename.endsWith('_x64-setup.nsis.zip')) return ['windows-x86_64']
	return null
}

function describe(filename) {
	const targets = updaterTargets(filename)
	if (targets) {
		return { kind: 'updater', platform: targets[0], targetPlatforms: targets, variant: 'tauri' }
	}
	if (filename.endsWith('.sig')) return { kind: 'signature', platform: null, targetPlatforms: [] }
	if (filename.endsWith('_x64_modern-setup.exe')) {
		return { kind: 'installer', platform: 'windows', targetPlatforms: [], variant: 'modern' }
	}
	if (filename.endsWith('_x64_nsis-setup.exe')) {
		return { kind: 'installer', platform: 'windows', targetPlatforms: [], variant: 'native' }
	}
	if (filename.endsWith('_x64-setup.exe')) {
		return { kind: 'installer', platform: 'windows', targetPlatforms: [], variant: 'legacy' }
	}
	if (filename.endsWith('_x64_portable.zip')) {
		return { kind: 'portable', platform: 'windows', targetPlatforms: [], variant: 'portable' }
	}
	if (filename.endsWith('.dmg')) {
		return { kind: 'installer', platform: 'macos', targetPlatforms: [], variant: 'dmg' }
	}
	if (filename.endsWith('.AppImage')) {
		return {
			kind: 'installer',
			platform: 'linux',
			targetPlatforms: [],
			variant: 'appimage',
		}
	}
	if (filename.endsWith('.deb')) {
		return { kind: 'installer', platform: 'linux', targetPlatforms: [], variant: 'deb' }
	}
	if (filename.endsWith('.rpm')) {
		return { kind: 'installer', platform: 'linux', targetPlatforms: [], variant: 'rpm' }
	}
	throw new Error(`Unrecognized release artifact ${filename}`)
}

function architecture(filename) {
	if (/universal/i.test(filename)) return 'universal'
	if (/(aarch64|arm64)/i.test(filename)) return 'aarch64'
	if (/(amd64|x86_64|x64)/i.test(filename)) return 'x86_64'
	return null
}

function sha256(filePath) {
	return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
}

const artifacts = files.map((filename) => {
	const filePath = path.join(assetsDir, filename)
	const descriptor = describe(filename)
	return {
		filename,
		size: fs.statSync(filePath).size,
		sha256: sha256(filePath),
		architecture: architecture(filename),
		...descriptor,
	}
})

const primaryArtifacts = artifacts.filter((artifact) => artifact.kind !== 'signature')
const artifactKeys = new Set()
for (const artifact of artifacts.filter((candidate) => candidate.kind === 'signature')) {
	const primary = primaryArtifacts.find(
		(candidate) => candidate.filename === artifact.filename.slice(0, -'.sig'.length),
	)
	if (!primary) throw new Error(`Signature has no matching release artifact ${artifact.filename}`)
	Object.assign(artifact, {
		platform: primary.platform,
		architecture: primary.architecture,
		targetPlatforms: primary.targetPlatforms,
		variant: primary.variant,
	})
}

for (const artifact of primaryArtifacts) {
	const signature = artifacts.find(
		(candidate) =>
			candidate.kind === 'signature' && candidate.filename === `${artifact.filename}.sig`,
	)
	artifact.signatureFilename = signature?.filename ?? null
	if (artifact.kind === 'updater' && !signature) {
		throw new Error(`Missing Tauri updater signature for ${artifact.filename}`)
	}
	const key = [artifact.kind, artifact.platform, artifact.architecture, artifact.variant].join(':')
	if (artifact.kind !== 'signature' && artifact.kind !== 'other' && artifactKeys.has(key)) {
		throw new Error(`Duplicate release artifact classification ${key}`)
	}
	artifactKeys.add(key)
}

fs.writeFileSync(
	outputPath,
	`${JSON.stringify({ version, artifacts: primaryArtifacts, files: artifacts }, null, 2)}\n`,
)
