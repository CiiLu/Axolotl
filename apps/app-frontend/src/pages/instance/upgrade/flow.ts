import type { ComputedRef, InjectionKey, MaybeRef, Ref } from 'vue'
import { computed, inject, provide, ref } from 'vue'

import type {
	InstanceUpgradePlan,
	InstanceUpgradeResult,
	InstanceUpgradeSolutionKind,
	InstanceUpgradeTargetEnvironment,
	SharedUpgradeMode,
} from '@/helpers/instance-upgrade'
import type { GameInstance } from '@/helpers/types'

export type UpgradeRouteRequirement = 'plan' | 'unblocked-plan' | 'selection' | 'job' | 'result'

export interface InstanceUpgradeFlow {
	instance: Readonly<Ref<GameInstance>>
	instanceId: Readonly<Ref<string>>
	targetEnvironment: Ref<InstanceUpgradeTargetEnvironment | null>
	plan: Ref<InstanceUpgradePlan | null>
	selectedSolutionKind: ComputedRef<InstanceUpgradeSolutionKind | null>
	createFullBackup: Ref<boolean>
	sharedUpgradeMode: Ref<SharedUpgradeMode | null>
	activeJobId: Ref<string | null>
	result: Ref<InstanceUpgradeResult | null>
	busy: Ref<boolean>
	error: Ref<unknown | null>
	reset: () => void
	clearPlan: () => void
	setTargetEnvironment: (environment: InstanceUpgradeTargetEnvironment | null) => void
	setPlan: (plan: InstanceUpgradePlan | null) => void
	setJob: (jobId: string | null) => void
	setResult: (result: InstanceUpgradeResult | null) => void
	hydrate: (snapshot: UpgradeFlowSnapshot) => void
	controls: Ref<UpgradeStepControls | null>
	registerStepControls: (controls: UpgradeStepControls | null) => void
}

export interface UpgradeStepControls {
	canNext: MaybeRef<boolean>
	busy?: MaybeRef<boolean>
	nextLabel: string
	onNext: () => void | Promise<void>
	onBack: () => void | Promise<void>
}


export interface UpgradeFlowSnapshot {
	instanceId: string
	returnFullPath: string
	targetEnvironment: InstanceUpgradeTargetEnvironment | null
	plan: InstanceUpgradePlan | null
	createFullBackup: boolean
	sharedUpgradeMode: SharedUpgradeMode | null
	activeJobId: string | null
	result: InstanceUpgradeResult | null
	scrollTop?: number
}

export const INSTANCE_UPGRADE_FLOW_KEY: InjectionKey<InstanceUpgradeFlow> = Symbol('instance-upgrade-flow')

export function provideUpgradeFlow(flow: InstanceUpgradeFlow) {
	provide(INSTANCE_UPGRADE_FLOW_KEY, flow)
}

export function provideInstanceUpgradeFlow(
	instance: Readonly<Ref<GameInstance>>,
): InstanceUpgradeFlow {
	const instanceId = computed(() => instance.value.id)
	const targetEnvironment = ref<InstanceUpgradeTargetEnvironment | null>(null)
	const plan = ref<InstanceUpgradePlan | null>(null)
	const createFullBackup = ref(true)
	const sharedUpgradeMode = ref<SharedUpgradeMode | null>(null)
	const activeJobId = ref<string | null>(null)
	const result = ref<InstanceUpgradeResult | null>(null)
	const busy = ref(false)
	const error = ref<unknown | null>(null)
	const selectedSolutionKind = computed(() => plan.value?.selectedSolution?.kind ?? null)
	const controls = ref<UpgradeStepControls | null>(null)

	function clearPlan() {
		plan.value = null
		activeJobId.value = null
		result.value = null
	}

	function reset() {
		targetEnvironment.value = null
		clearPlan()
		createFullBackup.value = true
		sharedUpgradeMode.value = null
		busy.value = false
		error.value = null
	}

	function hydrate(snapshot: UpgradeFlowSnapshot) {
		if (snapshot.instanceId !== instance.value.id) return
		targetEnvironment.value = snapshot.targetEnvironment
		plan.value = snapshot.plan
		createFullBackup.value = snapshot.createFullBackup
		sharedUpgradeMode.value = snapshot.sharedUpgradeMode
		activeJobId.value = snapshot.activeJobId
		result.value = snapshot.result
	}

	const flow: InstanceUpgradeFlow = {
		instance,
		instanceId,
		targetEnvironment,
		plan,
		selectedSolutionKind,
		createFullBackup,
		sharedUpgradeMode,
		activeJobId,
		result,
		busy,
		error,
		reset,
		clearPlan,
		setTargetEnvironment: (environment) => (targetEnvironment.value = environment),
		setPlan: (nextPlan) => (plan.value = nextPlan),
		setJob: (jobId) => (activeJobId.value = jobId),
		setResult: (nextResult) => (result.value = nextResult),
		hydrate,
		controls,
		registerStepControls: (next) => (controls.value = next),
	}
	provideUpgradeFlow(flow)
	return flow
}

export function useInstanceUpgradeFlow(): InstanceUpgradeFlow {
	const flow = inject(INSTANCE_UPGRADE_FLOW_KEY)
	if (!flow) throw new Error('Instance upgrade flow was not provided')
	return flow
}

export function isUpgradeRouteAvailable(
	requirement: UpgradeRouteRequirement | undefined,
	flow: InstanceUpgradeFlow,
): boolean {
	switch (requirement) {
		case 'plan':
			return flow.plan.value !== null
		case 'unblocked-plan':
			return flow.plan.value !== null && flow.plan.value.blockingIssues.length === 0
		case 'selection':
			return flow.plan.value?.selectedSolution != null
		case 'job':
			return flow.activeJobId.value !== null
		case 'result':
			return flow.result.value !== null
		default:
			return true
	}
}
