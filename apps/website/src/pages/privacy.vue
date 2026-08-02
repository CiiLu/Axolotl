<script setup lang="ts">
import LegalDocument from '~/components/ui/LegalDocument.vue'

const title = '隐私政策 - Axolotl Launcher'
const description = '了解 Axolotl Launcher 官方网站和桌面应用如何处理数据。'

useSeoMeta({
	title,
	description,
	ogTitle: title,
	ogDescription: description,
	ogType: 'website',
	ogUrl: 'https://www.axlmc.org/privacy',
	robots: 'index, follow',
})

useHead({
	link: [{ rel: 'canonical', href: 'https://www.axlmc.org/privacy' }],
})
</script>

<template>
	<LegalDocument
		eyebrow="Privacy Policy"
		title="隐私政策"
		description="本政策说明 Axolotl Launcher 官方网站和桌面应用处理数据的方式，以及第三方服务可能接收的信息。"
		updated-at="2026 年 8 月 2 日"
	>
		<h2>1. 适用范围</h2>
		<p>
			本政策适用于 www.axlmc.org 官方网站和 Axolotl Launcher 桌面应用。通过启动器访问的第三方网站、API、认证服务、服务器及内容下载地址，适用对应第三方自己的隐私政策。
		</p>

		<h2>2. 官方网站处理的数据</h2>
		<p>网站无需注册账户。为保存你的显示和下载偏好，浏览器会在本地保存：</p>
		<ul>
			<li>语言偏好 Cookie（<code>locale</code>）；</li>
			<li>主题、高级渲染、减少动态效果和外部链接打开方式；</li>
			<li>下载源选择（自动、CNB 或 GitHub）。</li>
		</ul>
		<p>
			这些偏好主要保存在 Cookie 或浏览器 localStorage 中，你可通过浏览器设置清除。网站不使用广告 Cookie，也未集成项目自有的用户行为分析服务。
		</p>
		<p>
			网站托管和网络基础设施提供者可能为保障访问、安全和故障排查而处理常规请求信息，例如 IP 地址、访问时间、请求路径、User-Agent 和错误日志。此类处理受对应提供者政策约束。
		</p>

		<h2>3. 桌面应用处理的数据</h2>
		<p>启动器会在你的设备本地处理并保存运行所需的数据，包括：</p>
		<ul>
			<li>应用设置、界面偏好和下载配置；</li>
			<li>Minecraft 实例、游戏版本、模组及其他内容的元数据；</li>
			<li>账户资料、登录会话、离线账户信息和自定义认证服务地址；</li>
			<li>游戏日志、崩溃信息、缓存、下载记录及更新状态。</li>
		</ul>
		<p>
			这些数据用于登录、启动游戏、管理内容、诊断故障和提供你主动使用的功能。项目当前不向维护者运营的分析服务上传遥测事件。开发者调试日志可能包含本地路径、实例或账户相关信息，公开分享前请先检查并删除敏感内容。
		</p>

		<h2>4. 与第三方共享和传输</h2>
		<p>
			当你使用登录、搜索、下载、更新、皮肤或联机等功能时，启动器会按你的操作直接连接第三方服务。请求可能包含服务完成操作所需的账户令牌、用户标识、项目或文件标识、IP 地址、设备和网络请求信息。可能涉及的服务包括：
		</p>
		<ul>
			<li>Microsoft 和 Minecraft 的登录、资料及游戏服务；</li>
			<li>你选择的第三方 Yggdrasil 认证服务；</li>
			<li>Modrinth、CurseForge 及内容作者提供的 API 和下载地址；</li>
			<li>GitHub、CNB 及其他用于版本检查、更新或文件分发的服务；</li>
			<li>你主动连接的 Minecraft 服务器或其他外部链接。</li>
		</ul>
		<p>
			项目维护者不会出售你的个人信息。第三方如何保存和使用请求数据，由其各自的隐私政策决定。
		</p>

		<h2>5. 数据保存与删除</h2>
		<p>
			网站偏好保存在你的浏览器中；启动器数据主要保存在你的设备中。你可以清除浏览器站点数据，或在启动器内移除账户、实例和缓存。卸载应用不一定自动删除全部数据目录，请在确认备份需求后手动清理残留文件。
		</p>
		<p>
			第三方服务保存的数据须通过对应服务的账户设置或隐私渠道管理。项目维护者无法代你访问、导出或删除第三方持有的数据。
		</p>

		<h2>6. 数据安全</h2>
		<p>
			我们通过开源审查、系统权限和安全更新等方式降低风险，但无法保证绝对安全。请从官方渠道下载软件、及时更新、保护系统账户，不要公开登录令牌、完整日志或包含个人信息的截图。
		</p>

		<h2>7. 未成年人</h2>
		<p>
			未成年人应在监护人指导下使用本项目，并遵守所在地区的年龄要求及 Minecraft、Microsoft 和其他相关服务的规则。请勿通过公开反馈渠道提交不必要的个人信息。
		</p>

		<h2>8. 政策变更</h2>
		<p>
			本政策可能因功能、第三方服务或法律要求变化而更新。新版本将在本页面发布，并标明最后更新日期。重大变化会尽量通过官方网站、GitHub 仓库或版本说明告知。
		</p>

		<h2>9. 联系方式</h2>
		<p>
			如有隐私问题或发现安全风险，请通过
			<a href="https://github.com/Mystic-Stars/Axolotl/issues" target="_blank" rel="noopener">GitHub Issues</a>
			联系项目维护者。请勿在公开 Issue 中提交账户令牌、密码、完整日志或其他敏感信息。也可加入官方 QQ 群 955605306 咨询一般问题。
		</p>
	</LegalDocument>
</template>
