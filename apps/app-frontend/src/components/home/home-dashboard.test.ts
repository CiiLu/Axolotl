import assert from 'node:assert/strict'
import test from 'node:test'

import {
	addHomeWidget,
	createDefaultHomeDashboard,
	createHomeDashboardPreset,
	createHomeDashboardSaveQueue,
	getHomeGridColumnCount,
	getHomeWidgetSpan,
	moveHomeWidget,
	normalizeHomeDashboard,
	packHomeWidgets,
	removeHomeWidget,
	replaceHomeDashboardWidgets,
	resizeHomeWidget,
	setHomeGreetingOptions,
	setHomeRecentLimit,
} from './home-dashboard.ts'

test('derives one to four columns from the dashboard container width', () => {
	assert.equal(getHomeGridColumnCount(0), 1)
	assert.equal(getHomeGridColumnCount(495), 1)
	assert.equal(getHomeGridColumnCount(496), 2)
	assert.equal(getHomeGridColumnCount(752), 3)
	assert.equal(getHomeGridColumnCount(2000), 4)
})

test('temporarily clamps wide widgets without changing their preferred size', () => {
	const config = createDefaultHomeDashboard()
	const preferredSize = config.widgets[0].size
	assert.deepEqual(getHomeWidgetSpan('2x2', 1), { columns: 1, rows: 2 })
	assert.deepEqual(getHomeWidgetSpan('2x2', 2), { columns: 2, rows: 2 })
	packHomeWidgets(config.widgets, 1)
	packHomeWidgets(config.widgets, 4)
	assert.equal(config.widgets[0].size, preferredSize)
})

test('adds, reorders, resizes, and removes independent placements', () => {
	const original = createDefaultHomeDashboard()
	const pinnedInstances = original.widgets.find((widget) => widget.kind === 'pinned-instances')!
	const duplicate = { ...pinnedInstances, id: 'duplicate-pinned-instances' }
	const added = addHomeWidget(original, duplicate)
	assert.equal(added.widgets.length, original.widgets.length + 1)
	assert.equal(added.widgets.filter((widget) => widget.kind === 'pinned-instances').length, 2)

	const moved = moveHomeWidget(added, added.widgets.length - 1, -1)
	assert.equal(moved.widgets.at(-2)?.id, duplicate.id)
	const resized = resizeHomeWidget(moved, duplicate.id, '1x1')
	assert.equal(resized.widgets.find((widget) => widget.id === duplicate.id)?.size, '1x1')
	const removed = removeHomeWidget(resized, duplicate.id)
	assert.deepEqual(removed.widgets, original.widgets)
})

test('accepts draggable order and restores the complete default layout', () => {
	const original = createDefaultHomeDashboard()
	const reversed = replaceHomeDashboardWidgets(original, [...original.widgets].reverse())
	assert.deepEqual(
		reversed.widgets.map((widget) => widget.id),
		original.widgets.map((widget) => widget.id).reverse(),
	)
	assert.deepEqual(
		createDefaultHomeDashboard().widgets.map(({ kind, size }) => ({ kind, size })),
		[
			{ kind: 'greeting', size: '2x1' },
			{ kind: 'recent', size: '2x2' },
			{ kind: 'calendar', size: '1x2' },
			{ kind: 'pinned-instances', size: '2x2' },
			{ kind: 'pinned-worlds', size: '1x2' },
			{ kind: 'pinned-servers', size: '1x2' },
		],
	)
})

test('creates the saved balanced preset with fresh placement ids', () => {
	const first = createHomeDashboardPreset('balanced')
	const second = createHomeDashboardPreset('balanced')

	assert.deepEqual(
		first.widgets.map(({ kind, size, options }) => ({
			kind,
			size,
			recentLimit: options?.recentLimit,
		})),
		[
			{ kind: 'greeting', size: '2x1', recentLimit: undefined },
			{ kind: 'calendar', size: '1x2', recentLimit: undefined },
			{ kind: 'recent', size: '2x2', recentLimit: 4 },
			{ kind: 'pinned-worlds', size: '1x2', recentLimit: undefined },
			{ kind: 'pinned-servers', size: '2x1', recentLimit: undefined },
			{ kind: 'pinned-instances', size: '2x2', recentLimit: undefined },
		],
	)
	assert.notDeepEqual(
		first.widgets.map((widget) => widget.id),
		second.widgets.map((widget) => widget.id),
	)
})

test('rolls back the latest failed save and reports the error', async () => {
	const original = createDefaultHomeDashboard()
	const changed = removeHomeWidget(original, original.widgets[0].id)
	let current = changed
	const errors: unknown[] = []
	const queue = createHomeDashboardSaveQueue(
		async () => {
			throw new Error('save failed')
		},
		(config) => {
			current = config
		},
		(error) => errors.push(error),
	)

	await queue.enqueue(changed, original)
	await queue.flush()
	assert.deepEqual(current, original)
	assert.equal(errors.length, 1)
})

test('normalizes a saved layout when entering Home again', () => {
	const saved = createDefaultHomeDashboard()
	const restored = normalizeHomeDashboard(JSON.parse(JSON.stringify(saved)))
	assert.deepEqual(restored, saved)
})

test('packs widgets into the earliest available cells', () => {
	const config = createDefaultHomeDashboard()
	const packed = packHomeWidgets(config.widgets, 4)
	assert.deepEqual(
		packed.map(({ column, row, effectiveColumns, effectiveRows }) => ({
			column,
			row,
			effectiveColumns,
			effectiveRows,
		})),
		[
			{ column: 1, row: 1, effectiveColumns: 2, effectiveRows: 1 },
			{ column: 3, row: 1, effectiveColumns: 2, effectiveRows: 2 },
			{ column: 1, row: 2, effectiveColumns: 1, effectiveRows: 2 },
			{ column: 2, row: 3, effectiveColumns: 2, effectiveRows: 2 },
			{ column: 4, row: 3, effectiveColumns: 1, effectiveRows: 2 },
			{ column: 1, row: 4, effectiveColumns: 1, effectiveRows: 2 },
		],
	)
})

test('normalizes malformed sizes while retaining duplicate widgets', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		widgets: [
			{ id: 'one', kind: 'calendar', size: '9x9' },
			{ id: 'two', kind: 'calendar', size: '1x1' },
			{ id: 'three', kind: 'world', size: '1x1', target: { instanceId: 'a' } },
		],
	})

	assert.deepEqual(
		normalized?.widgets.map(({ kind, size }) => ({ kind, size })),
		[
			{ kind: 'calendar', size: '1x2' },
			{ kind: 'calendar', size: '1x2' },
		],
	)
})

test('normalizes every persisted calendar placement to the fixed 1x2 size', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		widgets: [
			{ id: 'small', kind: 'calendar', size: '1x1' },
			{ id: 'wide', kind: 'calendar', size: '2x2' },
		],
	})

	assert.deepEqual(
		normalized?.widgets.map((widget) => widget.size),
		['1x2', '1x2'],
	)
})

test('normalizes every persisted greeting placement to the fixed 2x1 size', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		widgets: [
			{ id: 'small', kind: 'greeting', size: '1x1' },
			{
				id: 'tall',
				kind: 'greeting',
				size: '1x2',
				options: { greetingMode: 'text', greetingText: '  Ready to play  ' },
			},
		],
	})

	assert.deepEqual(
		normalized?.widgets.map((widget) => widget.size),
		['2x1', '2x1'],
	)
	assert.deepEqual(
		normalized?.widgets.map((widget) => widget.options),
		[
			{ greetingMode: 'greeting', greetingFont: 'sans', greetingFontSize: 22 },
			{
				greetingMode: 'text',
				greetingText: 'Ready to play',
				greetingFont: 'sans',
				greetingFontSize: 22,
			},
		],
	)
	assert.deepEqual(
		resizeHomeWidget(normalized!, 'small', '1x1').widgets.map((widget) => widget.size),
		['2x1', '2x1'],
	)
	assert.deepEqual(
		setHomeGreetingOptions(normalized!, 'small', 'text-and-greeting', '  Hi  ', 'minecraft', 29)
			.widgets[0].options,
		{
			greetingMode: 'text-and-greeting',
			greetingText: 'Hi',
			greetingFont: 'minecraft',
			greetingFontSize: 29,
		},
	)
})

test('normalizes greeting font settings and clamps font size', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		widgets: [
			{
				id: 'large',
				kind: 'greeting',
				size: '2x1',
				options: { greetingFont: 'serif', greetingFontSize: 80 },
			},
			{
				id: 'invalid',
				kind: 'greeting',
				size: '2x1',
				options: { greetingFont: 'comic-sans', greetingFontSize: 'large' },
			},
		],
	})!

	assert.deepEqual(
		normalized.widgets.map((widget) => ({
			font: widget.options?.greetingFont,
			fontSize: widget.options?.greetingFontSize,
		})),
		[
			{ font: 'serif', fontSize: 32 },
			{ font: 'sans', fontSize: 22 },
		],
	)
})

test('normalizes and updates recently played item limits', () => {
	const normalized = normalizeHomeDashboard({
		version: 1,
		widgets: [
			{ id: 'legacy', kind: 'recent', size: '2x2' },
			{ id: 'valid', kind: 'recent', size: '2x1', options: { recentLimit: 8 } },
			{ id: 'invalid', kind: 'recent', size: '1x2', options: { recentLimit: 99 } },
		],
	})!

	assert.deepEqual(
		normalized.widgets.map((widget) => widget.options?.recentLimit),
		[4, 8, 4],
	)
	assert.equal(setHomeRecentLimit(normalized, 'legacy', 6).widgets[0].options?.recentLimit, 6)
})
