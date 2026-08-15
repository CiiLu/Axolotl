import type { Config } from 'tailwindcss'

function oklchColor(variable: string) {
	return (({ opacityValue }: { opacityValue?: string }) =>
		opacityValue === undefined
			? `oklch(var(--${variable}))`
			: `oklch(var(--${variable}) / ${opacityValue})`) as unknown as string
}

export default {
	darkMode: ['class'],
	content: [
		'./app.vue',
		'./components/**/*.{vue,ts}',
		'./composables/**/*.ts',
		'./layouts/**/*.vue',
		'./pages/**/*.vue',
	],
	theme: {
		extend: {
			colors: {
				surface: {
					1: oklchColor('surface-1'),
					2: oklchColor('surface-2'),
					3: oklchColor('surface-3'),
					4: oklchColor('surface-4'),
					5: oklchColor('surface-5'),
				},
				border: oklchColor('border'),
				input: oklchColor('input'),
				ring: oklchColor('ring'),
				background: oklchColor('background'),
				foreground: oklchColor('foreground'),
				primary: {
					DEFAULT: oklchColor('primary'),
					foreground: oklchColor('primary-foreground'),
				},
				secondary: {
					DEFAULT: oklchColor('secondary'),
					foreground: oklchColor('secondary-foreground'),
				},
				muted: {
					DEFAULT: oklchColor('muted'),
					foreground: oklchColor('muted-foreground'),
				},
				accent: {
					DEFAULT: oklchColor('accent'),
					foreground: oklchColor('accent-foreground'),
				},
				destructive: {
					DEFAULT: oklchColor('destructive'),
					foreground: oklchColor('destructive-foreground'),
				},
				card: {
					DEFAULT: oklchColor('card'),
					foreground: oklchColor('card-foreground'),
				},
				popover: {
					DEFAULT: oklchColor('popover'),
					foreground: oklchColor('popover-foreground'),
				},
			},
			borderRadius: {
				sm: 'calc(var(--radius) - 4px)',
				md: 'calc(var(--radius) - 2px)',
				lg: 'var(--radius)',
				xl: 'calc(var(--radius) + 4px)',
				'2xl': 'calc(var(--radius) + 8px)',
			},
			fontFamily: {
				sans: [
					'Inter',
					'PingFang SC',
					'Microsoft YaHei',
					'ui-sans-serif',
					'system-ui',
					'sans-serif',
				],
				mono: ['JetBrains Mono', 'ui-monospace', 'SFMono-Regular', 'monospace'],
			},
		},
	},
	plugins: [],
} satisfies Config
