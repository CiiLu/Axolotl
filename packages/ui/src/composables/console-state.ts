import { ref } from 'vue'

export interface ConsoleState {
	output: { value: string }
	addLegacyLog: (log: string) => void
	clear: () => void
}

export function createConsoleState(): ConsoleState {
	const output = ref('')

	function addLegacyLog(log: string) {
		output.value += log
	}

	function clear() {
		output.value = ''
	}

	return {
		output,
		addLegacyLog,
		clear,
	}
}
