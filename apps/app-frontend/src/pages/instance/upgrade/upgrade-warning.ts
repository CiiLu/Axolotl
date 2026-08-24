import type { InstanceUpgradeIssueCode, InstanceUpgradeResult } from '@/helpers/instance-upgrade'

export interface UpgradeWarningRow {
	key: string
	code: InstanceUpgradeIssueCode | null
	relativePath: string | null
	legacyMessage: string | null
}

export function upgradeWarningMessageId(code: InstanceUpgradeIssueCode): string {
	return `instance.upgrade.warning.${code.replaceAll('_', '-')}`
}

export function upgradeResultWarningRows(result: InstanceUpgradeResult): UpgradeWarningRow[] {
	if (result.compatibilityWarningDetails !== undefined) {
		return result.compatibilityWarningDetails.map((warning, index) => ({
			key: `${warning.code}:${warning.contentId ?? warning.relativePath ?? index}`,
			code: warning.code,
			relativePath: warning.relativePath,
			legacyMessage: null,
		}))
	}
	return result.compatibilityWarnings.map((warning, index) => ({
		key: `${warning.code}:${warning.contentId ?? index}`,
		code: null,
		relativePath: null,
		legacyMessage: warning.message || warning.code,
	}))
}
