<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui/src/composables/i18n.ts'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	features: { id: 'axolotl-site.navigation.features', defaultMessage: 'Features' },
	faq: { id: 'axolotl-site.navigation.faq', defaultMessage: 'FAQ' },
	downloads: { id: 'axolotl-site.footer.downloads', defaultMessage: 'Downloads' },
	changelog: { id: 'axolotl-site.footer.release-history', defaultMessage: 'Changelog' },
	terms: { id: 'axolotl-site.navigation.terms', defaultMessage: 'Terms of Service' },
	privacy: { id: 'axolotl-site.navigation.privacy', defaultMessage: 'Privacy Policy' },
	developer: { id: 'axolotl-site.footer.developer', defaultMessage: 'Axolotl Launcher Team' },
	openSourcePrefix: {
		id: 'axolotl-site.footer.open-source-prefix',
		defaultMessage: 'Axolotl is',
	},
	openSourceLink: {
		id: 'axolotl-site.footer.open-source-link',
		defaultMessage: 'open source',
	},
	disclaimer: {
		id: 'axolotl-site.footer.minecraft-disclaimer',
		defaultMessage:
			'NOT AN OFFICIAL MINECRAFT SERVICE. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.',
	},
})
</script>

<template>
	<footer class="site-footer">
		<div class="footer-inner">
			<div class="footer-legal">
				<p>© {{ new Date().getFullYear() }} {{ formatMessage(messages.developer) }}</p>
				<p class="open-source">
					{{ formatMessage(messages.openSourcePrefix) }}
					<a href="https://github.com/Mystic-Stars/Axolotl" target="_blank" rel="noopener">
						{{ formatMessage(messages.openSourceLink) }} </a
					>.
				</p>
			</div>

			<nav class="footer-links" aria-label="Footer">
				<NuxtLink to="/#features">{{ formatMessage(messages.features) }}</NuxtLink>
				<NuxtLink to="/#faq">{{ formatMessage(messages.faq) }}</NuxtLink>
				<a href="#download">{{ formatMessage(messages.downloads) }}</a>
				<NuxtLink to="/changelog">{{ formatMessage(messages.changelog) }}</NuxtLink>
				<NuxtLink to="/terms">{{ formatMessage(messages.terms) }}</NuxtLink>
				<NuxtLink to="/privacy">{{ formatMessage(messages.privacy) }}</NuxtLink>
			</nav>

			<p class="disclaimer">{{ formatMessage(messages.disclaimer) }}</p>
		</div>
	</footer>
</template>

<style scoped lang="scss">
.site-footer {
	position: relative;
	border-top: 1px solid var(--color-divider);
	background: var(--surface-1);
	color: var(--color-base);

	&::before {
		position: absolute;
		top: -1px;
		left: 50%;
		width: min(76rem, calc(100% - 3rem));
		height: 1px;
		transform: translateX(-50%);
		background: linear-gradient(90deg, transparent, var(--color-brand), transparent);
		content: '';
		opacity: 0.6;
	}
}

.footer-inner {
	display: grid;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: center;
	gap: 0.8rem 2.5rem;
	width: min(76rem, calc(100% - 3rem));
	margin: 0 auto;
	padding: 1.75rem 0;
}

.footer-legal {
	display: flex;
	flex-wrap: wrap;
	gap: 0.4rem 1.4rem;
	color: var(--color-secondary);
	font-size: 0.78rem;

	p {
		margin: 0;
	}

	a {
		color: var(--color-base);
		text-underline-offset: 0.15rem;
	}
}

.footer-links {
	display: flex;
	flex-wrap: wrap;
	justify-content: flex-end;
	gap: 0.35rem 1.3rem;

	a {
		color: var(--color-secondary);
		font-size: 0.82rem;
		font-weight: 500;
		text-decoration: none;
		transition: color 140ms ease;

		&:hover {
			color: var(--color-brand);
		}
	}
}

.disclaimer {
	grid-column: 1 / -1;
	margin: 0;
	color: var(--color-secondary);
	font-size: 0.64rem;
	letter-spacing: 0.015em;
	text-align: center;
	opacity: 0.45;
}

@media (max-width: 900px) {
	.footer-inner {
		grid-template-columns: 1fr;
	}

	.footer-links {
		justify-content: flex-start;
	}
}

@media (max-width: 560px) {
	.footer-inner {
		width: calc(100% - 2rem);
	}
}
</style>
