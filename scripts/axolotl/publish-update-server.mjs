import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

const [catalogPath, releasePath, tag] = process.argv.slice(2)
const serverUrl = process.env.UPDATE_SERVER_URL?.replace(/\/$/, '')
const uploadToken = process.env.UPDATE_SERVER_UPLOAD_TOKEN
const webhookSecret = process.env.UPDATE_SERVER_WEBHOOK_SECRET

if (!catalogPath || !releasePath || !tag || !serverUrl || !uploadToken || !webhookSecret) {
	throw new Error(
		'Usage: UPDATE_SERVER_URL=... UPDATE_SERVER_UPLOAD_TOKEN=... UPDATE_SERVER_WEBHOOK_SECRET=... node scripts/axolotl/publish-update-server.mjs <catalog.json> <release.json> <version-tag>',
	)
}

const catalog = JSON.parse(fs.readFileSync(catalogPath, 'utf8'))
const release = JSON.parse(fs.readFileSync(releasePath, 'utf8'))
const version = tag.replace(/^v/, '')
if (catalog.version !== version) throw new Error('Artifact catalog version does not match the release tag')

const assetsDir = path.dirname(catalogPath)

for (const artifact of catalog.files) {
	const filePath = path.join(assetsDir, artifact.filename)
	const response = await fetch(
		`${serverUrl}/api/artifacts/${encodeURIComponent(version)}/${encodeURIComponent(artifact.filename)}`,
		{
			method: 'PUT',
			headers: {
				'X-Upload-Token': uploadToken,
				'X-Axolotl-SHA256': artifact.sha256,
				'X-Axolotl-Size': String(artifact.size),
				'X-Axolotl-Kind': artifact.kind,
				'X-Axolotl-Platform': artifact.platform ?? '',
				'X-Axolotl-Architecture': artifact.architecture ?? '',
				'X-Axolotl-Variant': artifact.variant ?? '',
				'Content-Type': 'application/octet-stream',
			},
			body: fs.readFileSync(filePath),
		},
	)
	if (!response.ok) {
		throw new Error(`Update Server upload failed for ${artifact.filename}: ${response.status} ${await response.text()}`)
	}
}

const artifacts = catalog.artifacts.map((artifact) => ({
	...artifact,
	signature: artifact.signatureFilename
		? fs.readFileSync(path.join(assetsDir, artifact.signatureFilename), 'utf8').trim()
		: null,
}))
const payload = JSON.stringify({
	event_id: `github-${tag}-${release.id ?? release.node_id ?? version}`,
	tag,
	version,
	channel: version.includes('-') ? 'beta' : 'release',
	release_id: String(release.id ?? ''),
	notes: release.body ?? '',
	published_at: new Date().toISOString(),
	force_update: process.env.UPDATE_SERVER_FORCE_UPDATE === 'true',
	artifacts,
})
const timestamp = String(Math.floor(Date.now() / 1000))
const signature = crypto
	.createHmac('sha256', webhookSecret)
	.update(`${timestamp}.${payload}`)
	.digest('hex')
const response = await fetch(`${serverUrl}/api/webhook/release`, {
	method: 'POST',
	headers: {
		'Content-Type': 'application/json',
		'X-Webhook-Timestamp': timestamp,
		'X-Webhook-Signature': `sha256=${signature}`,
	},
	body: payload,
})
if (!response.ok) {
	throw new Error(`Update Server publish failed: ${response.status} ${await response.text()}`)
}
