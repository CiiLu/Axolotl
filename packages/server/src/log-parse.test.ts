import assert from 'node:assert/strict'
import { test } from 'node:test'

import { classifyServerLogLine, summarizeServerExit } from './log-parse.ts'
import { computeServerStatus } from './status.ts'
import { requiredJavaMajorVersion, resolveServerJar } from './server-types.ts'

test('maps legacy game versions to their required Java major', () => {
	assert.equal(requiredJavaMajorVersion('1.21.4'), 21)
	assert.equal(requiredJavaMajorVersion('1.20.5'), 21)
	assert.equal(requiredJavaMajorVersion('1.20.4'), 17)
	assert.equal(requiredJavaMajorVersion('1.17.1'), 17)
	assert.equal(requiredJavaMajorVersion('1.16.5'), 8)
	assert.equal(requiredJavaMajorVersion('1.12.2'), 8)
})

test('maps year-based game versions to Java 25', () => {
	assert.equal(requiredJavaMajorVersion('26.2'), 25)
	assert.equal(requiredJavaMajorVersion('26.1'), 25)
	assert.equal(requiredJavaMajorVersion('26w14a'), 25)
	assert.equal(requiredJavaMajorVersion('25w46a'), 21)
	assert.equal(requiredJavaMajorVersion('unknown'), 25)
})

test('detects the eula notice in server logs', () => {
	assert.equal(
		classifyServerLogLine('[ServerMain/ERROR]: Failed to start the minecraft server').eulaRequired,
		undefined,
	)
	assert.equal(
		classifyServerLogLine(
			'[ServerMain/ERROR]: You need to agree to the EULA in order to run the server. Go to eula.txt for more info.',
		).eulaRequired,
		true,
	)
})

test('detects a fully started server', () => {
	assert.equal(
		classifyServerLogLine('[Server thread/INFO]: Done (3.542s)! For help, type "help"').started,
		true,
	)
})

test('first-run eula exit is not a crash', () => {
	const lines = [
		'Starting minecraft server version 1.21',
		'You need to agree to the EULA in order to run the server.',
	]
	assert.deepEqual(summarizeServerExit(lines, 1), { crashed: false, eulaRequired: true })
	assert.deepEqual(summarizeServerExit(['Done (1.0s)!'], 0), {
		crashed: false,
		eulaRequired: false,
	})
	assert.deepEqual(summarizeServerExit(['Exception in thread "main"'], 1), {
		crashed: true,
		eulaRequired: false,
	})
})

test('resolves vanilla server jar from version info', () => {
	const jar = resolveServerJar('vanilla', {
		gameVersion: '1.21.4',
		vanillaVersionInfo: {
			downloads: { server: { sha1: 'abc', size: 100, url: 'https://example.com/server.jar' } },
		},
	})
	assert.equal(jar?.url, 'https://example.com/server.jar')
	assert.equal(jar?.filename, 'server.jar')
})

test('resolves fabric server launcher url', () => {
	const jar = resolveServerJar('fabric', {
		gameVersion: '1.21.4',
		loaderVersion: '0.16.9',
		installerVersion: '1.0.3',
	})
	assert.equal(jar?.url, 'https://meta.fabricmc.net/v2/versions/loader/1.21.4/0.16.9/1.0.3/jar')
})

test('installer-based types resolve to null until implemented', () => {
	assert.equal(resolveServerJar('forge', { gameVersion: '1.21.4' }), null)
	assert.equal(resolveServerJar('neoforge', { gameVersion: '1.21.4' }), null)
})

test('computes server status precedence', () => {
	const base = {
		manifest: { id: 'a' },
		isRunning: false,
		isStarting: false,
		lastExitWasCrash: false,
		eulaAccepted: false,
		eulaFileExists: false,
	}
	assert.equal(computeServerStatus(base), 'created')
	assert.equal(computeServerStatus({ ...base, eulaFileExists: true }), 'eula_pending')
	assert.equal(computeServerStatus({ ...base, eulaAccepted: true }), 'ready')
	assert.equal(computeServerStatus({ ...base, isStarting: true }), 'starting')
	assert.equal(computeServerStatus({ ...base, isRunning: true }), 'running')
	assert.equal(
		computeServerStatus({ ...base, lastExitWasCrash: true, eulaAccepted: true }),
		'crashed',
	)
})
