import assert from 'node:assert/strict'
import test from 'node:test'

import {
	getLastBrowseContentDisplayMode,
	getLastBrowseContentProjectType,
	setLastBrowseContentDisplayMode,
	setLastBrowseContentProjectType,
} from './browse-display-mode.ts'

const storageKey = 'axolotl-browse-content-display-mode'
const projectTypeStorageKey = 'axolotl-browse-content-project-type'
const originalStorageDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')

function installMemoryStorage() {
	const values = new Map<string, string>()
	Object.defineProperty(globalThis, 'localStorage', {
		configurable: true,
		value: {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => values.set(key, value),
		},
	})
	return values
}

function restoreStorage() {
	if (originalStorageDescriptor) {
		Object.defineProperty(globalThis, 'localStorage', originalStorageDescriptor)
	} else {
		delete (globalThis as { localStorage?: Storage }).localStorage
	}
}

test('browse display mode persists valid values and falls back to the list', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getLastBrowseContentDisplayMode(), 'list')

		setLastBrowseContentDisplayMode('compact')
		assert.equal(getLastBrowseContentDisplayMode(), 'compact')

		setLastBrowseContentDisplayMode('grid')
		assert.equal(getLastBrowseContentDisplayMode(), 'grid')

		values.set(storageKey, 'invalid')
		assert.equal(getLastBrowseContentDisplayMode(), 'list')
	} finally {
		restoreStorage()
	}
})

test('browse project type persists resources and rejects non-content routes', () => {
	const values = installMemoryStorage()

	try {
		assert.equal(getLastBrowseContentProjectType(), 'modpack')

		setLastBrowseContentProjectType('mod')
		assert.equal(getLastBrowseContentProjectType(), 'mod')

		setLastBrowseContentProjectType('shader')
		assert.equal(getLastBrowseContentProjectType(), 'shader')

		values.set(projectTypeStorageKey, 'server')
		assert.equal(getLastBrowseContentProjectType(), 'modpack')
	} finally {
		restoreStorage()
	}
})
