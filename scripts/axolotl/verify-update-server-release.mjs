const [tag] = process.argv.slice(2)
const serverUrl = process.env.UPDATE_SERVER_URL?.replace(/\/$/, '')

if (!tag || !serverUrl) {
	throw new Error(
		'Usage: UPDATE_SERVER_URL=... node scripts/axolotl/verify-update-server-release.mjs <version-tag>',
	)
}

const version = tag.replace(/^v/, '')
const channel = version.includes('-') ? 'beta' : 'release'

async function fetchHeadWithRetry(url, expectedSize) {
	const attempts = 5
	let lastStatus = 'unknown error'
	for (let attempt = 0; attempt < attempts; attempt++) {
		try {
			const response = await fetch(url, { method: 'HEAD', cache: 'no-store' })
			const contentLength = Number(response.headers.get('content-length'))
			if (
				response.ok &&
				contentLength > 0 &&
				(expectedSize === undefined || contentLength === expectedSize)
			) {
				return response
			}
			lastStatus = `${response.status} content-length=${response.headers.get('content-length') ?? 'missing'}`
		} catch (error) {
			lastStatus = error instanceof Error ? error.message : String(error)
		}
		if (attempt < attempts - 1) await new Promise((resolve) => setTimeout(resolve, 2000))
	}
	throw new Error(`${url}: ${lastStatus}`)
}

const response = await fetch(`${serverUrl}/latest`, {
	headers: {
		Accept: 'application/json',
		'X-Axolotl-Channel': channel,
		'X-Axolotl-Platform': 'windows-x86_64',
		'X-Axolotl-Version': '0.0.0',
	},
})
if (!response.ok) {
	throw new Error(
		`Update Server latest verification failed: ${response.status} ${await response.text()}`,
	)
}
const latest = await response.json()
if (latest.version !== version || typeof latest.force_update !== 'boolean') {
	throw new Error(`Update Server returned an unexpected manifest for ${version}`)
}
const update = latest.platforms?.['windows-x86_64']
if (!update?.signature || !update?.url) {
	throw new Error('Update Server manifest is missing the Windows updater artifact')
}
const updateUrl = new URL(update.url)
if (updateUrl.protocol !== 'https:' || !updateUrl.pathname.startsWith(`/dist/${version}/`)) {
	throw new Error(`Unexpected Update Server updater URL: ${update.url}`)
}
await fetchHeadWithRetry(update.url)
const downloads = await fetch(`${serverUrl}/api/downloads/${encodeURIComponent(version)}`)
if (!downloads.ok) {
	throw new Error(`Update Server download catalog verification failed: ${downloads.status}`)
}
const catalog = await downloads.json()
if (
	catalog.version !== version ||
	!Array.isArray(catalog.downloads) ||
	catalog.downloads.length === 0
) {
	throw new Error(`Update Server download catalog is incomplete for ${version}`)
}
for (const artifact of catalog.downloads) {
	const url = new URL(artifact.url)
	if (url.protocol !== 'https:' || !url.pathname.startsWith(`/dist/${version}/`)) {
		throw new Error(`Unexpected complete package URL: ${artifact.url}`)
	}
	try {
		await fetchHeadWithRetry(artifact.url, artifact.size)
	} catch {
		throw new Error(`Complete package verification failed for ${artifact.filename}`)
	}
}
