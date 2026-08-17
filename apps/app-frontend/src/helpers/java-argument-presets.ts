import { defineMessage } from '@modrinth/ui'

import { createGcPresets } from '@/helpers/gc/gc-presets'
import type { GcContext, JavaArgumentPreset } from '@/helpers/gc/types'
import {
	FALLEN_AUTH_PROXY_BLOG_URL,
	FALLEN_AUTH_PROXY_JAVA_ARGS_STRING,
} from '@/helpers/java-arguments'

export type { JavaArgumentPreset }

export const JAVA_ARGUMENT_PRESETS: JavaArgumentPreset[] = [
	{
		id: 'mojang-auth-mirror',
		title: defineMessage({
			id: 'app.java-arguments.presets.auth-mirror.title',
			defaultMessage: 'Authentication service mirror',
		}),
		description: defineMessage({
			id: 'app.java-arguments.presets.auth-mirror.description',
			defaultMessage:
				'HTTP forwarding for the Mojang authentication servers hosted by Fallen-Breath.',
		}),
		args: FALLEN_AUTH_PROXY_JAVA_ARGS_STRING,
		link: FALLEN_AUTH_PROXY_BLOG_URL,
	},
]

export function getJavaArgumentPresets(gcContext?: GcContext): JavaArgumentPreset[] {
	return [...JAVA_ARGUMENT_PRESETS, ...createGcPresets(gcContext)]
}

export function getPresetsByGroup(
	presets: JavaArgumentPreset[],
): Map<string | undefined, JavaArgumentPreset[]> {
	const groups = new Map<string | undefined, JavaArgumentPreset[]>()
	for (const preset of presets) {
		const group = preset.group
		if (!groups.has(group)) {
			groups.set(group, [])
		}
		groups.get(group)!.push(preset)
	}
	return groups
}
