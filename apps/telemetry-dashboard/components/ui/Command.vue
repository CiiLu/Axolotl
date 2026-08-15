<script setup lang="ts">
import { BarChart3, CircleAlert, Search, Server } from 'lucide-vue-next'
import {
	DialogContent,
	DialogDescription,
	DialogOverlay,
	DialogPortal,
	DialogRoot,
	DialogTitle,
} from 'reka-ui'

const open = defineModel<boolean>('open', { required: true })
const query = ref('')
const router = useRouter()
const items = [
	{ label: '数据总览', path: '/', icon: BarChart3 },
	{ label: '错误分析', path: '/errors', icon: CircleAlert },
	{ label: '系统状态', path: '/system', icon: Server },
]
const filtered = computed(() =>
	items.filter((item) => item.label.toLowerCase().includes(query.value.toLowerCase())),
)

async function select(path: string): Promise<void> {
	open.value = false
	query.value = ''
	await router.push(path)
}
</script>

<template>
	<DialogRoot v-model:open="open">
		<DialogPortal>
			<DialogOverlay class="sheet-overlay fixed inset-0 z-40 bg-black/45 backdrop-blur-[1px]" />
			<DialogContent
				class="command-panel fixed left-1/2 top-[18vh] z-50 w-[calc(100vw-2rem)] max-w-lg -translate-x-1/2 overflow-hidden rounded-xl border border-surface-5 bg-popover shadow-2xl outline-none"
			>
				<DialogTitle class="sr-only">快速跳转</DialogTitle>
				<DialogDescription class="sr-only">搜索控制台页面</DialogDescription>
				<div class="flex items-center gap-2 border-b px-4">
					<Search class="size-4 shrink-0 text-muted-foreground" />
					<input
						v-model="query"
						autofocus
						placeholder="输入页面名称..."
						class="h-12 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
					/>
				</div>
				<div class="p-2">
					<p v-if="!filtered.length" class="px-3 py-10 text-center text-sm text-muted-foreground">
						没有匹配的页面
					</p>
					<button
						v-for="item in filtered"
						:key="item.path"
						type="button"
						class="flex w-full cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-left text-sm outline-none hover:bg-surface-3 focus-visible:bg-surface-3"
						@click="select(item.path)"
					>
						<component :is="item.icon" class="size-4 shrink-0 text-muted-foreground" />
						{{ item.label }}
					</button>
				</div>
			</DialogContent>
		</DialogPortal>
	</DialogRoot>
</template>
