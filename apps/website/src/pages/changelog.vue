<script setup lang="ts">
import { CalendarIcon, HistoryIcon } from '@modrinth/assets'
import Accordion from '@modrinth/ui/src/components/base/Accordion.vue'
import ButtonStyled from '@modrinth/ui/src/components/base/ButtonStyled.vue'
import TagItem from '@modrinth/ui/src/components/base/TagItem.vue'
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

import {
	RELEASE_CHANGE_TYPES,
	type ReleaseChangeType,
	type ReleaseNotesLocale,
	parseReleaseNotes,
} from '~/utils/release-notes'

type GitHubRelease = {
	id: number
	draft: boolean
	prerelease: boolean
	tag_name: string
	name: string | null
	body: string | null
	published_at: string | null
}

const GITHUB_RELEASES_URL = 'https://api.github.com/repos/Mystic-Stars/Axolotl/releases'
const RELEASES_PER_PAGE = 100
const FIRST_RELEASE_VERSION = [1, 4, 0] as const

const { formatMessage, locale } = useVIntl()

const messages = defineMessages({
	seoTitle: {
		id: 'axolotl-site.changelog.seo.title',
		defaultMessage: 'Changelog - Axolotl Launcher',
	},
	seoDescription: {
		id: 'axolotl-site.changelog.seo.description',
		defaultMessage: 'See what changed in each public Axolotl Launcher release.',
	},
	eyebrow: { id: 'axolotl-site.changelog.eyebrow', defaultMessage: 'Release history' },
	title: { id: 'axolotl-site.changelog.title', defaultMessage: 'Changelog' },
	description: {
		id: 'axolotl-site.changelog.description',
		defaultMessage: 'Browse features, changes, and fixes in every public release.',
	},
	loading: {
		id: 'axolotl-site.changelog.loading',
		defaultMessage: 'Checking published releases…',
	},
	errorTitle: {
		id: 'axolotl-site.changelog.error.title',
		defaultMessage: 'Changelog is temporarily unavailable',
	},
	errorDescription: {
		id: 'axolotl-site.changelog.error.description',
		defaultMessage:
			'We could not fetch the release history. Your network may be unavailable, or the GitHub API request limit for this network may have been reached.',
	},
	retry: { id: 'axolotl-site.changelog.retry', defaultMessage: 'Retry' },
	empty: {
		id: 'axolotl-site.changelog.empty',
		defaultMessage: 'No public release notes are available yet.',
	},
	noReleaseNotes: {
		id: 'axolotl-site.changelog.no-release-notes',
		defaultMessage: 'No release notes were provided for this version.',
	},
	added: { id: 'axolotl-site.changelog.category.added', defaultMessage: 'Added' },
	changed: { id: 'axolotl-site.changelog.category.changed', defaultMessage: 'Changed' },
	deprecated: {
		id: 'axolotl-site.changelog.category.deprecated',
		defaultMessage: 'Deprecated',
	},
	removed: { id: 'axolotl-site.changelog.category.removed', defaultMessage: 'Removed' },
	fixed: { id: 'axolotl-site.changelog.category.fixed', defaultMessage: 'Fixed' },
	security: { id: 'axolotl-site.changelog.category.security', defaultMessage: 'Security' },
})

const {
	data: releases,
	error,
	status,
	refresh,
} = await useAsyncData(
	'axolotl-github-releases',
	async () => {
		const allReleases: GitHubRelease[] = []

		for (let page = 1; ; page++) {
			const releasePage = await $fetch<GitHubRelease[]>(GITHUB_RELEASES_URL, {
				query: { per_page: RELEASES_PER_PAGE, page },
			})

			allReleases.push(...releasePage)

			if (releasePage.length < RELEASES_PER_PAGE) break
		}

		return allReleases.filter(
			(release) =>
				!release.draft &&
				!release.prerelease &&
				isReleaseAtLeast(release.tag_name, FIRST_RELEASE_VERSION),
		)
	},
	{ server: false },
)

function isReleaseAtLeast(tagName: string, minimumVersion: readonly number[]) {
	const match = tagName.match(/^v?(\d+)\.(\d+)\.(\d+)/)
	if (!match) return false

	const version = match.slice(1).map(Number)

	for (const [index, part] of version.entries()) {
		if (part > minimumVersion[index]) return true
		if (part < minimumVersion[index]) return false
	}

	return true
}

function getReleaseTitle(release: GitHubRelease) {
	return release.name?.trim() || release.tag_name
}

function getReleaseDate(publishedAt: string | null) {
	return publishedAt?.slice(0, 10) ?? ''
}

const categoryClasses: Record<ReleaseChangeType, string> = {
	added: 'bg-brand-green',
	changed: 'bg-brand-blue',
	deprecated: 'bg-brand-orange',
	removed: 'bg-brand-red',
	fixed: 'bg-brand-purple',
	security: 'bg-brand-orange',
}

const releaseNotesLocale = computed<ReleaseNotesLocale>(() =>
	locale.value === 'zh-CN' ? 'zh-CN' : 'en-US',
)
const releaseCategories = computed(() =>
	(releases.value ?? []).map((release) => {
		const notes = parseReleaseNotes(release.body)[releaseNotesLocale.value]

		return {
			id: release.id,
			categories: RELEASE_CHANGE_TYPES.flatMap((type) => {
				const changes = notes[type]
				if (!changes?.length) return []

				return [
					{
						type,
						label: formatMessage(messages[type]),
						className: categoryClasses[type],
						changes,
					},
				]
			}),
		}
	}),
)

function getReleaseCategories(releaseId: number) {
	return releaseCategories.value.find((release) => release.id === releaseId)?.categories ?? []
}

const isLoading = computed(() => status.value === 'idle' || status.value === 'pending')
const seoTitle = computed(() => formatMessage(messages.seoTitle))
const seoDescription = computed(() => formatMessage(messages.seoDescription))

useSeoMeta({
	title: () => seoTitle.value,
	description: () => seoDescription.value,
	ogTitle: () => seoTitle.value,
	ogDescription: () => seoDescription.value,
	ogType: 'website',
	ogUrl: 'https://www.axlmc.org/changelog',
	robots: 'index, follow',
})

useHead({
	link: [{ rel: 'canonical', href: 'https://www.axlmc.org/changelog' }],
})
</script>

<template>
	<section class="changelog-page">
		<header class="changelog-header">
			<span class="section-eyebrow">{{ formatMessage(messages.eyebrow) }}</span>
			<h1>{{ formatMessage(messages.title) }}</h1>
			<p>{{ formatMessage(messages.description) }}</p>
		</header>

		<div v-if="isLoading" class="status-panel" role="status">
			<div class="loading-indicator" aria-hidden="true" />
			{{ formatMessage(messages.loading) }}
		</div>

		<div v-else-if="error" class="status-panel error-panel" role="alert">
			<div>
				<h2>{{ formatMessage(messages.errorTitle) }}</h2>
				<p>{{ formatMessage(messages.errorDescription) }}</p>
			</div>
			<ButtonStyled color="brand" type="outlined">
				<button type="button" @click="refresh()">{{ formatMessage(messages.retry) }}</button>
			</ButtonStyled>
		</div>

		<p v-else-if="!releases?.length" class="status-panel">
			{{ formatMessage(messages.empty) }}
		</p>

		<div v-else class="announcement-list">
			<Accordion
				v-for="(release, index) in releases"
				:key="release.id"
				:open-by-default="index === 0"
				class="announcement"
				button-class="group flex w-full cursor-pointer items-center gap-4 border-0 bg-transparent px-5 py-4 text-left"
			>
				<template #title>
					<div class="announcement-heading">
						<div class="announcement-title-row">
							<h2>{{ getReleaseTitle(release) }}</h2>
							<TagItem>{{ release.tag_name }}</TagItem>
						</div>
						<div class="announcement-date">
							<CalendarIcon aria-hidden="true" />
							<time :datetime="release.published_at ?? undefined">
								{{ getReleaseDate(release.published_at) }}
							</time>
						</div>
					</div>
				</template>

				<div class="announcement-content">
					<p v-if="getReleaseCategories(release.id).length === 0" class="no-release-notes">
						{{ formatMessage(messages.noReleaseNotes) }}
					</p>
					<section
						v-for="(category, categoryIndex) in getReleaseCategories(release.id)"
						:key="category.type"
						class="change-group"
						:class="{ 'first-change-group': categoryIndex === 0 }"
					>
						<h3>
							<span :class="category.className" aria-hidden="true" />
							{{ category.label }}
						</h3>
						<ul>
							<li v-for="change in category.changes" :key="change">{{ change }}</li>
						</ul>
					</section>
				</div>
			</Accordion>
		</div>

		<div class="changelog-footer">
			<HistoryIcon aria-hidden="true" />
			<a href="https://github.com/Mystic-Stars/Axolotl/releases" target="_blank" rel="noopener">
				GitHub Releases
			</a>
		</div>
	</section>
</template>

<style scoped lang="scss">
.changelog-page {
	width: min(52rem, calc(100% - 2rem));
	margin: 0 auto;
	padding: 4rem 0 5rem;
}

.changelog-header {
	max-width: 40rem;
	margin-bottom: 2.5rem;

	h1 {
		margin: 0.5rem 0 0;
		color: var(--color-contrast);
		font-size: 2.25rem;
		line-height: 1.15;
	}

	p {
		margin: 1rem 0 0;
		color: var(--color-secondary);
		line-height: 1.65;
	}
}

.announcement-list {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.announcement {
	overflow: hidden;
	border: 1px solid var(--surface-5);
	border-radius: 0.5rem;
	background: var(--surface-4);
}

.announcement-heading {
	display: flex;
	min-width: 0;
	flex: 1;
	align-items: center;
	justify-content: space-between;
	gap: 1rem;
}

.announcement-title-row,
.announcement-date,
.changelog-footer {
	display: flex;
	align-items: center;
}

.announcement-title-row {
	min-width: 0;
	gap: 0.75rem;

	h2 {
		margin: 0;
		overflow: hidden;
		color: var(--color-contrast);
		font-size: 1rem;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
}

.announcement-date {
	flex-shrink: 0;
	gap: 0.35rem;
	color: var(--color-secondary);
	font-size: 0.8125rem;

	svg {
		width: 1rem;
		height: 1rem;
	}
}

.announcement-content {
	padding: 0 1.25rem 0.5rem;
	border-top: 1px solid var(--surface-5);
	background: var(--surface-3);
}

.no-release-notes {
	margin: 0;
	padding: 1rem 0 0.5rem;
	color: var(--color-secondary);
	line-height: 1.6;
}

.change-group {
	display: grid;
	grid-template-columns: 7rem minmax(0, 1fr);
	gap: 1.25rem;
	padding: 1rem 0;
	border-top: 1px solid var(--surface-5);

	&.first-change-group {
		border-top: 0;
	}

	h3 {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
		color: var(--color-secondary);
		font-size: 0.875rem;
		font-weight: 600;

		span {
			width: 0.5rem;
			height: 0.5rem;
			border-radius: 50%;
		}
	}

	ul {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin: 0;
		padding-left: 1.25rem;
		color: var(--color-base);
		line-height: 1.6;
		overflow-wrap: anywhere;
	}
}

.status-panel {
	display: flex;
	align-items: center;
	justify-content: center;
	gap: 0.75rem;
	margin: 0;
	padding: 2rem;
	border: 1px solid var(--surface-5);
	border-radius: 0.5rem;
	background: var(--surface-4);
	color: var(--color-secondary);
	text-align: center;
}

.error-panel {
	justify-content: space-between;
	text-align: left;

	h2,
	p {
		margin: 0;
	}

	h2 {
		color: var(--color-contrast);
		font-size: 1rem;
	}

	p {
		margin-top: 0.25rem;
	}
}

.loading-indicator {
	width: 1rem;
	height: 1rem;
	border: 2px solid var(--surface-5);
	border-top-color: var(--color-brand);
	border-radius: 50%;
	animation: spin 700ms linear infinite;
}

.changelog-footer {
	justify-content: center;
	gap: 0.5rem;
	margin-top: 2rem;
	color: var(--color-secondary);
	font-size: 0.875rem;

	svg {
		width: 1rem;
		height: 1rem;
	}

	a {
		color: inherit;
	}
}

@keyframes spin {
	to {
		transform: rotate(1turn);
	}
}

@media (max-width: 600px) {
	.changelog-page {
		padding: 2.5rem 0 3rem;
	}

	.changelog-header h1 {
		font-size: 1.875rem;
	}

	.announcement-heading,
	.error-panel,
	.change-group {
		align-items: flex-start;
		flex-direction: column;
	}

	.change-group {
		grid-template-columns: 1fr;
		gap: 0.5rem;
	}
}
</style>
