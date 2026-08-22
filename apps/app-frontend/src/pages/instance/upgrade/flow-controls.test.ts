import assert from 'node:assert/strict'
import test from 'node:test'
import { computed, ref } from 'vue'

import { upgradeControlEnabled } from './flow-controls.ts'

test('registered upgrade control reads live ref values without re-registration', () => {
	const canPlan = ref(false)
	const control = computed(() => canPlan.value)
	assert.equal(upgradeControlEnabled(control), false)
	canPlan.value = true
	assert.equal(upgradeControlEnabled(control), true)
})

test('missing controls remain disabled', () => {
	assert.equal(upgradeControlEnabled(undefined), false)
})
