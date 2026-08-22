import { toValue } from 'vue'
import type { MaybeRef } from 'vue'

export function upgradeControlEnabled(value: MaybeRef<boolean> | undefined): boolean {
	return toValue(value ?? false)
}
