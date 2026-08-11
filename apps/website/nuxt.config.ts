import svgLoader from 'vite-svg-loader'

const SITE_URL = 'https://axlmc.org'

export default defineNuxtConfig({
	srcDir: 'src/',
	app: {
		head: {
			htmlAttrs: {
				class: 'accent-pink dark-mode',
				lang: 'zh-CN',
			},
			title: 'Axolotl Launcher - 免费开源的 Minecraft 启动器',
			// 在 body 渲染前同步应用已保存的主题偏好，避免浅色用户首屏闪深色
			script: [
				{
					key: 'theme-init',
					innerHTML: `(function(){try{var t=localStorage.getItem('axolotl-theme');var r=t;if(!r||r==='system'){r=(window.matchMedia&&window.matchMedia('(prefers-color-scheme: light)').matches)?'light':'dark'}var d=document.documentElement;d.classList.remove('light-mode','dark-mode','oled-mode');d.classList.add(r==='light'?'light-mode':r==='oled'?'oled-mode':'dark-mode');d.style.colorScheme=r==='light'?'light':'dark'}catch(e){}})()`,
				},
			],
			link: [
				{ rel: 'icon', type: 'image/png', href: '/axolotl.png' },
				{ rel: 'apple-touch-icon', type: 'image/png', href: '/axolotl.png' },
			],
		},
	},
	runtimeConfig: {
		public: {
			siteUrl: SITE_URL,
		},
	},
	vite: {
		css: {
			preprocessorOptions: {
				scss: {
					silenceDeprecations: ['import'],
				},
			},
		},
		resolve: {
			dedupe: ['vue'],
		},
		plugins: [
			svgLoader({
				svgoConfig: {
					plugins: [
						{
							name: 'preset-default',
							params: {
								overrides: {
									removeViewBox: false,
									cleanupIds: { minify: false },
								},
							},
						},
					],
				},
			}),
		],
	},
	css: ['~/assets/styles/tailwind.css'],
	postcss: {
		plugins: {
			tailwindcss: {},
			autoprefixer: {},
		},
	},
	nitro: {
		// 显式锁定静态 preset：Netlify 构建环境存在 NETLIFY 变量时 Nitro 会
		// 自动切换 netlify preset 并生成 `_redirects: /* /404.html 404` 通配
		// 规则，该文件规则优先于 netlify.toml，会把 /api/* 转发全部吞成 404。
		preset: 'static',
		// preset: 'static' 会把产物输出到 .output/public，这里显式指回 dist，
		// 保持与 Netlify UI 中配置的 Publish directory (apps/website/dist) 一致。
		output: { publicDir: '../dist' },
		prerender: {
			crawlLinks: false,
			routes: ['/', '/changelog', '/terms', '/privacy'],
		},
	},
	routeRules: {
		'/': { static: true },
		'/changelog': { static: true },
		'/terms': { static: true },
		'/privacy': { static: true },
	},
	typescript: {
		shim: false,
		strict: true,
		typeCheck: false,
	},
	compatibilityDate: '2025-01-01',
	telemetry: false,
})
