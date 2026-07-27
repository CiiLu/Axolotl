<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'

import DropzoneFileInput from '../../../base/DropzoneFileInput.vue'
import { injectCreationFlowContext } from '../creation-flow-context'

const ctx = injectCreationFlowContext()
const { formatMessage } = useVIntl()

// ── Launcher icons (3, arc arrangement) ──
// @ts-ignore — Vite resolves .ico as static asset URL
import pcl2CeUrl from '@modrinth/assets/icons/PCL2_CE.ico'
// @ts-ignore — Vite resolves .ico as static asset URL
import pcl2Url from '@modrinth/assets/icons/PCL2.ico'
// @ts-ignore — Vite resolves .ico as static asset URL
import hmclUrl from '@modrinth/assets/icons/HMCL.ico'

const launcherIcons = [
	{ key: 'pcl2ce', url: pcl2CeUrl, alt: 'PCL2 CE' },
	{ key: 'pcl2', url: pcl2Url, alt: 'PCL2' },
	{ key: 'hmcl', url: hmclUrl, alt: 'HMCL' },
]

const messages = defineMessages({
	dropZoneClick: {
		id: 'creation-flow.modal.import-instance.drop-zone.click',
		defaultMessage: 'Click to select a file or drag & drop any file/folder',
	},
	importPrompt: {
		id: 'creation-flow.modal.import-instance.import-prompt',
		defaultMessage:
			'Drag & drop launcher folders, modpack files, or .minecraft folders to import an instance in one click',
	},
})

// ── Drop zone handler (via DropzoneFileInput) ──
function onDropzoneChange(paths: string[]) {
	console.log('[ImportInstanceStage] onDropzoneChange called with paths:', paths)
	if (!paths || paths.length === 0) {
		console.log('[ImportInstanceStage] no paths, returning')
		return
	}

	const filePath = paths[0]
	console.log('[ImportInstanceStage] filePath:', filePath)

	if (ctx.onImportFileReceived) {
		console.log('[ImportInstanceStage] calling ctx.onImportFileReceived with filePath')
		ctx.onImportFileReceived({
			file: null,
			filePath,
			source: 'file-picker',
		})
		return
	}

	// Fallback: set path directly on context
	console.log('[ImportInstanceStage] ctx.onImportFileReceived not set, using fallback')
	ctx.modpackFile.value = null
	ctx.modpackFilePath.value = filePath
	if (ctx.finishDisabled.value) {
		console.log('[ImportInstanceStage] finishDisabled is true, returning')
		return
	}
	if (ctx.flowType === 'instance') {
		console.log('[ImportInstanceStage] finish() called')
		ctx.finish()
	} else {
		console.log('[ImportInstanceStage] setting stage to final-config')
		ctx.modal.value?.setStage('final-config')
	}
}
</script>