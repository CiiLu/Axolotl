<script setup lang="ts">
import LegalDocument from '~/components/ui/LegalDocument.vue'

const title = '服务条款 - Axolotl Launcher'
const description = '使用 Axolotl Launcher 官方网站及桌面应用前，请阅读本服务条款。'

useSeoMeta({
	title,
	description,
	ogTitle: title,
	ogDescription: description,
	ogType: 'website',
	ogUrl: 'https://www.axlmc.org/terms',
	robots: 'index, follow',
})

useHead({
	link: [{ rel: 'canonical', href: 'https://www.axlmc.org/terms' }],
})
</script>

<template>
	<LegalDocument
		eyebrow="服务条款"
		title="服务条款"
		description="本条款说明 Axolotl Launcher 各部分适用的开源许可证、官方服务规则及第三方权利边界。"
		updated-at="2026 年 8 月 2 日"
	>
		<h2>1. 适用范围与文件效力</h2>
		<p>
			本条款适用于 Axolotl Launcher
			官方网站、官方发布的桌面应用及项目维护者提供的相关服务。软件的复制、修改、分发和网络部署，以仓库中对应文件的许可证与复制声明为准；本页面只是中文说明，不替代、修改或限制原始许可证。
		</p>
		<p>
			根据 GPL-3.0 和
			AGPL-3.0，仅接收或运行未经修改的软件副本不要求你接受许可证。只有在复制、修改、传播、分发软件，或以适用
			AGPL 的修改版本向网络用户提供服务时，许可证中的对应条件才适用。
		</p>

		<h2>2. 仓库采用的许可证</h2>
		<p>本仓库没有一份覆盖全部文件的统一许可证。不同部分采用不同许可：</p>
		<ul>
			<li>
				官方网站源代码采用
				<a
					href="https://github.com/Mystic-Stars/Axolotl/blob/main/apps/website/LICENSE"
					target="_blank"
					rel="noopener"
					>GNU Affero General Public License Version 3 only（AGPL-3.0-only）</a
				>；
			</li>
			<li>
				桌面应用相关 package 采用
				<a
					href="https://github.com/Mystic-Stars/Axolotl/blob/main/apps/app-frontend/LICENSE"
					target="_blank"
					rel="noopener"
					>GNU General Public License Version 3 only（GPL-3.0-only）</a
				>；
			</li>
			<li>
				其他 package 保留各自的上游许可证，应查看对应目录中的 <code>LICENSE</code>、<code
					>COPYING.md</code
				>
				和源码声明；
			</li>
			<li>
				第三方代码、数据、图标、图片、商标及其他素材可能采用单独许可，不因被收录在本仓库中而自动改用
				GPL 或 AGPL。
			</li>
		</ul>

		<h2>3. 你的开源软件权利</h2>
		<p>
			在遵守适用许可证的前提下，你可以运行、研究、复制、修改和再分发受 GPL-3.0-only 或 AGPL-3.0-only
			保护的代码，也可以收费分发。项目维护者不会通过本服务条款对这些许可证已经授予的权利增加限制。
		</p>
		<p>
			复制或分发时通常需要保留版权、许可证和无担保声明；分发修改版本时需要明确标注修改及日期，并按对应许可证向接收者提供完整许可权利与相应源代码。分发目标代码时，应按许可证规定提供可机器读取的对应源代码。不得对接收者行使许可证权利增加额外限制。
		</p>
		<p>
			若你修改采用 AGPL-3.0-only 的网站代码，并让用户通过网络与修改版本交互，应按 AGPL 第 13
			条向这些用户显著提供免费获取该版本对应源代码的机会。具体义务以许可证原文为准。
		</p>

		<h2>4. 品牌、商标与受限素材</h2>
		<p>
			开源软件许可证不自动授予商标权。Axolotl Launcher 是独立的非官方项目，与
			Mojang、Microsoft、Rinth, Inc.、Modrinth、CurseForge 或 MinecraftSearch
			不存在隶属、认可或背书关系。相关名称和商标归各自权利人所有。
		</p>
		<p>
			Modrinth 商标、徽标、封面和其他受限品牌素材不属于 Axolotl Launcher 品牌。网站复制声明列出的
			Modrinth 品牌元素未经 Rinth, Inc.
			明确书面许可不得使用。外部徽标须遵守各权利人的品牌规范。使用或再分发其他第三方图片、图标、数据和素材前，也应核对对应声明。
		</p>

		<h2>5. 特别归属与第三方许可</h2>
		<p>
			桌面启动器的中文内容搜索包含来自 Plain Craft Launcher 的未修改
			<code>WikiEntries.txt</code> 快照，该文件采用 PCL Limited Distribution License，并包含由 MC
			百科贡献的中文项目名称信息。种子地图功能还包含 MIT 许可的 cubiomes，以及权利归
			MinecraftSearch、Mojang 或各素材创作者所有的图像。它们不适用 Axolotl 桌面代码的 GPL-3.0-only
			授权，具体范围以仓库复制声明为准。
		</p>

		<h2>6. 官方网站与服务的使用</h2>
		<p>
			使用项目维护者运营的官方网站、下载源、反馈渠道或其他基础设施时，你应遵守适用法律及对应第三方平台规则，不得攻击、干扰、滥用基础设施，不得实施欺诈或侵害他人权利。这些服务规则只约束官方服务的使用，不限制
			GPL 或 AGPL 授予你对软件副本的权利。
		</p>
		<p>
			启动器会按你的操作连接 Microsoft、Minecraft、Modrinth、CurseForge、GitHub、CNB、自定义
			Yggdrasil
			服务及内容作者提供的下载地址。第三方服务、游戏内容和文件受其各自条款、隐私政策、许可证、可用性及地区限制约束；项目维护者不替第三方提供担保。
		</p>

		<h2>7. 无担保</h2>
		<p>
			按照 GPL-3.0 和 AGPL-3.0 第 15
			条，在适用法律允许的范围内，软件按“现状”提供，不附带任何明示或默示担保，包括适销性和特定用途适用性的默示担保。软件质量与性能风险由使用者承担；软件存在缺陷时，必要的维护、修复或更正费用由使用者承担，除非另有书面约定。
		</p>

		<h2>8. 责任限制</h2>
		<p>
			按照 GPL-3.0 和 AGPL-3.0 第 16
			条，除适用法律要求或另有书面约定外，版权人以及依许可证修改或分发软件的其他主体，不对因使用或无法使用软件产生的一般、特殊、附带或后果性损害负责，包括数据丢失、数据不准确、第三方损失或软件无法与其他程序协同工作。若当地法律不允许完整排除责任，则按许可证第
			17 条及当地法律处理。
		</p>

		<h2>9. 条款与许可证更新</h2>
		<p>
			本服务条款可随官方服务、第三方平台或法律要求更新。更新不会追溯撤销已经依适用开源许可证授予的权利，也不会把标注为“Version
			3
			only”的代码自动改为后续许可证版本。任何代码许可变更必须由相应权利人依法作出，并体现在仓库文件中。
		</p>

		<h2>10. 联系与完整文本</h2>
		<p>
			许可范围存在疑问时，请先查看仓库根目录及对应 package 的 <code>COPYING.md</code>、<code
				>LICENSE</code
			>
			和源码声明。如需进一步确认，请通过
			<a href="https://github.com/Mystic-Stars/Axolotl/issues" target="_blank" rel="noopener"
				>GitHub Issues</a
			>
			联系项目维护者，或加入官方 QQ 群 955605306。本页面不是法律意见。
		</p>
	</LegalDocument>
</template>
