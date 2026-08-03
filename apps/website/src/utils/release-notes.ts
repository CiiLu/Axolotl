export const RELEASE_CHANGE_TYPES = [
	'added',
	'changed',
	'deprecated',
	'removed',
	'fixed',
	'security',
] as const

export type ReleaseChangeType = (typeof RELEASE_CHANGE_TYPES)[number]
export type ReleaseNotesLocale = 'en-US' | 'zh-CN'
export type ReleaseNoteGroups = Partial<Record<ReleaseChangeType, string[]>>
export type ParsedReleaseNotes = Record<ReleaseNotesLocale, ReleaseNoteGroups>

const LOCALE_HEADINGS: Record<string, ReleaseNotesLocale> = {
	中文: 'zh-CN',
	english: 'en-US',
}

const CATEGORY_HEADINGS: Record<ReleaseNotesLocale, Record<string, ReleaseChangeType>> = {
	'zh-CN': {
		新增: 'added',
		变更: 'changed',
		弃用: 'deprecated',
		移除: 'removed',
		'bug 修复': 'fixed',
		修复: 'fixed',
		安全修复: 'security',
		安全: 'security',
	},
	'en-US': {
		added: 'added',
		changed: 'changed',
		deprecated: 'deprecated',
		removed: 'removed',
		fixed: 'fixed',
		fixes: 'fixed',
		'bug fixes': 'fixed',
		security: 'security',
	},
}

function normalizeHeading(heading: string) {
	return heading.trim().replace(/\s+/g, ' ').toLowerCase()
}

export function parseReleaseNotes(body: string | null): ParsedReleaseNotes {
	const notes: ParsedReleaseNotes = {
		'en-US': {},
		'zh-CN': {},
	}
	if (!body) return notes

	let locale: ReleaseNotesLocale | undefined
	let category: ReleaseChangeType | undefined

	for (const line of body.split(/\r?\n/)) {
		const heading = line.match(/^(#{2,3})\s+(.+?)\s*#*\s*$/)
		if (heading) {
			const [, hashes, text] = heading
			const normalizedHeading = normalizeHeading(text)

			if (hashes.length === 2) {
				locale = LOCALE_HEADINGS[normalizedHeading]
				category = undefined
			} else if (locale) {
				category = CATEGORY_HEADINGS[locale][normalizedHeading]
			}

			continue
		}

		const item = line.match(/^\s*(?:[-*+]\s+|\d+\.\s+)(.+?)\s*$/)
		if (!locale || !category || !item) continue

		const changes = notes[locale][category] ?? []
		changes.push(item[1])
		notes[locale][category] = changes
	}

	return notes
}
