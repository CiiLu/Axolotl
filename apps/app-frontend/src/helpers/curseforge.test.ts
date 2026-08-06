import assert from 'node:assert/strict'
import test from 'node:test'

import { getCurseForgeDownloadFailureDetails } from './curseforge.ts'

test('recognizes CurseForge download diagnostics without exposing them in the notification', () => {
	const details = getCurseForgeDownloadFailureDetails(
		new Error(
			'Network download error: connection failed\nDownload failed after 4/4 attempts. Recent attempt history:\n- attempt=4; url=https://mediafilez.forgecdn.net/files/example.jar; proxy=System; category=connect',
		),
	)

	assert.match(details ?? '', /forgecdn\.net/)
})

test('does not classify non-CurseForge download failures', () => {
	assert.equal(
		getCurseForgeDownloadFailureDetails(
			new Error(
				'Download failed after 4/4 attempts. Recent attempt history:\n- url=https://cdn.modrinth.com/data/example.jar',
			),
		),
		null,
	)
})
