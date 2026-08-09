import { defineMessage, type MessageDescriptor } from '@modrinth/ui'

import {
	FALLEN_AUTH_PROXY_BLOG_URL,
	FALLEN_AUTH_PROXY_JAVA_ARGS_STRING,
} from '@/helpers/java-arguments'

export interface JavaArgumentPreset {
	id: string
	title: MessageDescriptor
	description: MessageDescriptor
	args: string
	link: string
}

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
