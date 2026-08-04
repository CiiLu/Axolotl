<script setup lang="ts">
import {
	GameIcon,
	MoreVerticalIcon,
	PinIcon,
	PlayIcon,
	SpinnerIcon,
	StopCircleIcon,
	TimerIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	OverflowMenu,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import dayjs from 'dayjs'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import InstanceIcon from '@/components/ui/InstanceIcon.vue'
import { useMinecraftLaunchError } from '@/composables/useMinecraftLaunchError'
import { trackEvent } from '@/helpers/analytics'
import { kill, run } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { handleSevereError } from '@/store/error'

type InstanceCardLayout = 'spotlight' | 'row' | 'tile'

const props = withDefaults(
	defineProps<{
		instance: GameInstance
		pinned: boolean
		playing?: boolean
		layout?: InstanceCardLayout
	}>(),
	{
		playing: false,
		layout: 'row',
	},
)

const emit = defineEmits<{
	'pinned-change': [instance: GameInstance, pinned: boolean]
}>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatRelativeTime = useRelativeTime()
const handleMinecraftLaunchError = useMinecraftLaunchError()
const router = useRouter()
const loading = ref(false)

const messages = defineMessages({
	pin: { id: 'app.home.instances.pin', defaultMessage: 'Pin to Home' },
	unpin: { id: 'app.home.instances.unpin', defaultMessage: 'Unpin from Home' },
	play: { id: 'app.home.instances.play', defaultMessage: 'Play instance' },
	stop: { id: 'app.home.instances.stop', defaultMessage: 'Stop instance' },
	played: { id: 'app.instance.played', defaultMessage: 'Played {time}' },
	neverPlayed: { id: 'app.instance.never-played', defaultMessage: 'Never played' },
})

const menuOptions = computed(() => [
	{
		id: props.pinned ? 'unpin' : 'pin',
		action: () => emit('pinned-change', props.instance, !props.pinned),
	},
])
const versionLabel = computed(() => {
	const loader = props.instance.loader === 'vanilla' ? 'Minecraft' : props.instance.loader
	return `${loader} ${props.instance.game_version}`
})
const lastPlayed = computed(() =>
	props.instance.last_played
		? formatMessage(messages.played, {
				time: formatRelativeTime(dayjs(props.instance.last_played).toISOString()),
			})
		: formatMessage(messages.neverPlayed),
)
const iconSize = computed(() => (props.layout === 'spotlight' ? '52px' : '42px'))

async function openInstance() {
	await router.push(`/instance/${encodeURIComponent(props.instance.id)}`)
}

async function playInstance() {
	loading.value = true
	try {
		await run(props.instance.id)
		trackEvent('InstanceStart', {
			loader: props.instance.loader,
			game_version: props.instance.game_version,
			source: 'HomePinnedInstance',
		})
	} catch (error) {
		const handled = await handleMinecraftLaunchError(error, {
			instance_id: props.instance.id,
			instance_name: props.instance.name,
		})
		if (!handled) handleSevereError(error, { instanceId: props.instance.id })
	} finally {
		loading.value = false
	}
}

async function stopInstance() {
	await kill(props.instance.id).catch(handleError)
	trackEvent('InstanceStop', {
		loader: props.instance.loader,
		game_version: props.instance.game_version,
		source: 'HomePinnedInstance',
	})
}
</script>

<template>
	<div class="home-instance-card" :data-layout="layout">
		<button
			type="button"
			class="home-instance-main"
			:aria-label="instance.name"
			@click="openInstance"
		>
			<InstanceIcon
				:icon-path="instance.icon_path"
				:instance-id="instance.id"
				:alt="instance.name"
				:size="iconSize"
				class="home-instance-icon"
			/>
			<span class="home-instance-copy">
				<strong>{{ instance.name }}</strong>
				<span class="home-instance-version"><GameIcon /> {{ versionLabel }}</span>
				<span v-if="layout !== 'tile'" class="home-instance-played">
					<TimerIcon /> {{ lastPlayed }}
				</span>
			</span>
		</button>

		<div class="home-instance-actions" @click.stop>
			<ButtonStyled v-if="playing" circular size="small" color="red">
				<button v-tooltip="formatMessage(messages.stop)" @click="stopInstance">
					<StopCircleIcon />
				</button>
			</ButtonStyled>
			<ButtonStyled
				v-else
				circular
				size="small"
				:color="layout === 'spotlight' ? 'brand' : 'standard'"
			>
				<button v-tooltip="formatMessage(messages.play)" :disabled="loading" @click="playInstance">
					<SpinnerIcon v-if="loading" class="animate-spin" />
					<PlayIcon v-else class="translate-x-px" />
				</button>
			</ButtonStyled>
			<ButtonStyled circular size="small" type="transparent" class="home-instance-menu">
				<OverflowMenu
					:options="menuOptions"
					:tooltip="formatMessage(pinned ? messages.unpin : messages.pin)"
				>
					<MoreVerticalIcon />
					<template #pin><PinIcon /> {{ formatMessage(messages.pin) }}</template>
					<template #unpin>
						<PinIcon class="rotate-45" /> {{ formatMessage(messages.unpin) }}
					</template>
				</OverflowMenu>
			</ButtonStyled>
		</div>
	</div>
</template>

<style scoped>
.home-instance-card {
	display: grid;
	min-width: 0;
	grid-template-columns: minmax(0, 1fr) auto;
	align-items: center;
	gap: 0.375rem;
	padding: 0.5rem;
	border-radius: 6px;
	background: transparent;
	transition: background-color 120ms ease;
}

.home-instance-card:hover,
.home-instance-card:focus-within {
	background: color-mix(in srgb, var(--color-green) 8%, var(--color-button-bg));
}

.home-instance-main {
	display: grid;
	min-width: 0;
	grid-template-columns: auto minmax(0, 1fr);
	align-items: center;
	gap: 0.625rem;
	padding: 0;
	border: 0;
	background: transparent;
	color: inherit;
	text-align: left;
	cursor: pointer;
}

.home-instance-main:focus-visible {
	border-radius: 6px;
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}

.home-instance-icon {
	filter: saturate(0.92);
	transition: transform 140ms ease;
}

.home-instance-card:hover .home-instance-icon {
	transform: scale(1.04);
}

.home-instance-copy {
	display: flex;
	min-width: 0;
	flex-direction: column;
	gap: 0.125rem;
}

.home-instance-copy strong {
	overflow: hidden;
	color: var(--color-contrast);
	font-size: 0.875rem;
	line-height: 1.2;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-instance-version,
.home-instance-played {
	display: flex;
	min-width: 0;
	align-items: center;
	gap: 0.25rem;
	overflow: hidden;
	color: var(--color-secondary);
	font-size: 0.7rem;
	font-weight: 600;
	line-height: 1.2;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.home-instance-version svg,
.home-instance-played svg {
	width: 0.75rem;
	height: 0.75rem;
	flex: 0 0 auto;
}

.home-instance-actions {
	display: flex;
	align-items: center;
	gap: 0.125rem;
}

.home-instance-card[data-layout='spotlight'] {
	min-height: 5.5rem;
	padding: 0.625rem;
	background: color-mix(in srgb, var(--color-green) 7%, transparent);
}

.home-instance-card[data-layout='spotlight'] .home-instance-copy strong {
	font-size: 1rem;
}

.home-instance-card[data-layout='tile'] {
	grid-template-columns: minmax(0, 1fr);
	align-content: center;
	gap: 0.375rem;
	padding: 0.625rem;
}

.home-instance-card[data-layout='tile'] .home-instance-actions {
	justify-content: flex-end;
}

.home-instance-card[data-layout='tile'] .home-instance-menu {
	display: none;
}
</style>
